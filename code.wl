from "./runtime.c" {
    fn print(s: string): void
}

fn main() {
    let i = 0;
    while i < 5 {
        print("Hello world!\n");
        i = i + 1;
    }
}
