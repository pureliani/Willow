from "std/io" { print }

type Foo<T> = { value: T };

fn main() { 
    let x: Foo<string> = { value: "hello world" };
    print(x.value);
}
