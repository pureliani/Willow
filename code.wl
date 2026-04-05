from "./runtime.c" {
    fn print(s: string): void
}

fn main() {
    let msg = "Hello from Willow!";
    print(msg);
}