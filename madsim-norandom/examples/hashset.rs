use std::collections::HashSet;

fn main() {
    for _ in 0..9 {
        let set = (0..10).collect::<HashSet<i32>>();
        let seq = set.into_iter().collect::<Vec<_>>();
        println!("{:?}", seq);
    }
}
