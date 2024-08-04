use std::process::Command;

fn main() {
  let output = Command::new("npx")
        .arg("tailwindcss")
        .arg("-i")
        .arg("input.css") // replace with your actual input file
        .arg("-o")
        .arg("output.css") // replace with your actual output file
        .output()
        .expect("Failed to execute Tailwind CSS compiler");
  if !output.status.success() {
    panic!("Tailwind css compilation failed");
  }
}
