from "std/io" { print }

fn identity<T>(val: T): T {
    return val;
}

type HasId = { id: i32 };

fn print_id<T extends HasId>(item: T) {
    let id_val = item.id;
    let returned_id = identity(id_val);
}

fn main() {
    let user = { id: 42, name: "Alice" };
    print_id(user);

}
