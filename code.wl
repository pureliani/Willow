from "std/io" { print }


fn bar<T>(arg: T) {
    print(arg);
}

fn main() { 
    bar("hello");
}
