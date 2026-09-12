fn main() -> Result<(), Box<dyn std::error::Error>> {
    libdqg::copy_res_to_output_dir()?;
    Ok(())
}
