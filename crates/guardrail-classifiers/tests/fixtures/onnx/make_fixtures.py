"""Generate the tiny ONNX models used by guardrail-classifiers' `onnx` tests.

These are not trained models. Each one computes its logits from a single
rule: "does the input contain the token `trigger`?". That makes test
outcomes exact and lets CI exercise the real ONNX code path (tokenizer,
ONNX Runtime session, output parsing, thresholds, fail modes) without
downloading hundreds of MB of weights.

All outputs are committed next to this script. Regenerate with:

    pip install onnx==1.17.0
    python make_fixtures.py

Files:

- tokenizer.json         word-level tokenizer; "trigger" -> id 1, unknown -> 0
- keyword_2class.onnx    logits [1, 2] = [0, +10] with trigger, [0, -10] without.
                         Same output layout as a 2-class (SAFE, INJECTION) model.
- keyword_multilabel.onnx logits [1, 16]. Logit 0 ("toxicity") is +10 with
                         trigger, -4 without; logits 1..15 are -9. Same layout
                         as unitary/unbiased-toxic-roberta. The clean-input
                         values mirror a real multi-label model, where every
                         label is unlikely but "toxicity" is the least
                         unlikely.
- wrong_output.onnx      logits [1, 1]: a model with an unexpected output
                         shape, used to force an inference error.
- keyword_multilabel_max514.onnx
                         keyword_multilabel plus a 514-row position lookup,
                         like RoBERTa's position embeddings: inference fails
                         for inputs longer than 514 tokens.
"""

import json
from pathlib import Path

import onnx
from onnx import TensorProto, helper

HERE = Path(__file__).resolve().parent
TRIGGER_ID = 1
OPSET = 13
IR_VERSION = 8  # widely supported by ONNX Runtime builds


def inputs():
    return [
        helper.make_tensor_value_info("input_ids", TensorProto.INT64, [1, "seq"]),
        helper.make_tensor_value_info("attention_mask", TensorProto.INT64, [1, "seq"]),
    ]


def const(name, values, dtype=TensorProto.FLOAT, dims=None):
    dims = dims if dims is not None else []
    return helper.make_node(
        "Constant",
        [],
        [name],
        value=helper.make_tensor(name + "_value", dtype, dims, values),
    )


def trigger_present_nodes():
    """Nodes producing `present` [1, 1] float: 1.0 if any input id == TRIGGER_ID."""
    return [
        const("trigger_id", [TRIGGER_ID], TensorProto.INT64),
        helper.make_node("Equal", ["input_ids", "trigger_id"], ["is_trigger"]),
        helper.make_node("Cast", ["is_trigger"], ["is_trigger_f"], to=TensorProto.FLOAT),
        helper.make_node("ReduceMax", ["is_trigger_f"], ["present"], axes=[1], keepdims=1),
    ]


def affine(src, dst, scale, offset):
    """dst = src * scale + offset."""
    return [
        const(dst + "_scale", [scale]),
        const(dst + "_offset", [offset]),
        helper.make_node("Mul", [src, dst + "_scale"], [dst + "_scaled"]),
        helper.make_node("Add", [dst + "_scaled", dst + "_offset"], [dst]),
    ]


def save(nodes, n_outputs, filename):
    graph = helper.make_graph(
        nodes,
        filename.removesuffix(".onnx"),
        inputs(),
        [helper.make_tensor_value_info("logits", TensorProto.FLOAT, [1, n_outputs])],
    )
    model = helper.make_model(
        graph,
        producer_name="guardrail-rs test fixtures",
        opset_imports=[helper.make_opsetid("", OPSET)],
    )
    model.ir_version = IR_VERSION
    onnx.checker.check_model(model)
    onnx.save(model, HERE / filename)


def keyword_2class():
    nodes = trigger_present_nodes()
    nodes += affine("present", "injection", 20.0, -10.0)  # +10 / -10
    nodes += [
        const("safe", [0.0], dims=[1, 1]),
        helper.make_node("Concat", ["safe", "injection"], ["logits"], axis=1),
    ]
    save(nodes, 2, "keyword_2class.onnx")


def keyword_multilabel():
    nodes = trigger_present_nodes()
    nodes += affine("present", "toxicity", 14.0, -4.0)  # +10 / -4
    nodes += [
        const("others", [-9.0] * 15, dims=[1, 15]),
        helper.make_node("Concat", ["toxicity", "others"], ["logits"], axis=1),
    ]
    save(nodes, 16, "keyword_multilabel.onnx")


def keyword_multilabel_max514():
    # positions = Range(0, seq_len); Gather(table[514, 1], positions) fails
    # with "indices element out of data bounds" once seq_len > 514, the same
    # way RoBERTa's position-embedding Gather does. The gathered values sum to
    # at most 514e-6 and are added to the toxicity logit so the node isn't
    # pruned; that shift is far too small to change any test outcome.
    nodes = trigger_present_nodes()
    nodes += [
        # positions = CumSum(attention_mask) - 1, i.e. 0..seq_len-1, computed
        # the way RoBERTa does it. (Range + Gather would be rewritten by ONNX
        # Runtime's optimizer into a clamping Slice, hiding the limit.)
        const("axis1", [1], TensorProto.INT64),
        helper.make_node("CumSum", ["attention_mask", "axis1"], ["cumsum"]),
        const("one_i", [1], TensorProto.INT64),
        helper.make_node("Sub", ["cumsum", "one_i"], ["positions"]),
        # lookup into a 514-row table; out of bounds past 514 tokens. The
        # values are tiny but non-zero: with an all-zero table ONNX Runtime's
        # graph optimizer removes the lookup and the limit disappears.
        const("pos_table", [1e-6] * 514, dims=[514, 1]),
        helper.make_node("Gather", ["pos_table", "positions"], ["pos_emb"], axis=0),
        # scalar, so adding it to the [1, 1] toxicity logit keeps rank 2
        helper.make_node("ReduceSum", ["pos_emb"], ["pos_sum"], keepdims=0),
    ]
    nodes += affine("present", "toxicity_raw", 14.0, -4.0)
    nodes += [
        helper.make_node("Add", ["toxicity_raw", "pos_sum"], ["toxicity"]),
        const("others", [-9.0] * 15, dims=[1, 15]),
        helper.make_node("Concat", ["toxicity", "others"], ["logits"], axis=1),
    ]
    save(nodes, 16, "keyword_multilabel_max514.onnx")


def wrong_output():
    nodes = trigger_present_nodes()
    nodes += affine("present", "logits", 20.0, -10.0)
    save(nodes, 1, "wrong_output.onnx")


def tokenizer():
    vocab = {"[UNK]": 0, "trigger": TRIGGER_ID, "hello": 2, "world": 3}
    data = {
        "version": "1.0",
        "truncation": None,
        "padding": None,
        "added_tokens": [],
        "normalizer": {"type": "Lowercase"},
        "pre_tokenizer": {"type": "Whitespace"},
        "post_processor": None,
        "decoder": None,
        "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "[UNK]"},
    }
    (HERE / "tokenizer.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    tokenizer()
    keyword_2class()
    keyword_multilabel()
    keyword_multilabel_max514()
    wrong_output()
    print("wrote fixtures to", HERE)
