fn main() {
    println!("Hello world!");
}


#[cfg(test)]
mod tests {
    use temp_testdir::TempDir;

    #[test]
    fn default_tempdir_should_create_a_directory() {
        let temp = TempDir::default();
        println!("{}", temp.to_string_lossy());
        assert!(temp.is_dir())
    }
}
