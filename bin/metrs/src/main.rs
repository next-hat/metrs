fn main() {
  println!("In comming soon...");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_main() {
    main();
  }
}
