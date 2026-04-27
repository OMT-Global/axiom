fn banner(name: &str) -> String {
    format!("hello {}", name)
}

fn lucky(base: i32) -> i32 {
    base + 2
}

fn is_ready(value: i32) -> bool {
    value == 42
}

fn main() {
    let answer = lucky(40);
    let ready = is_ready(answer);
    if ready {
        println!("{}", banner("from stage1"));
    } else {
        println!("stage1 failed");
    }
    println!("{}", answer);
    println!("{}", ready);
}
