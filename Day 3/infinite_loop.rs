fn main() {
    let mut counter = 0;

    loop {
        counter += 1;
        println!("counter = {}", counter);

        if counter == 3 {
            break;
        }
    }
}
