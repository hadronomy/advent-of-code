use miette::*;

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    Ok(String::from(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "";
        assert_eq!("example", process(input)?);
        Ok(())
    }
}
