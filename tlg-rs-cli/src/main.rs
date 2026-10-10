use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use image::{ImageReader, ImageFormat};
use clap::{Subcommand, Parser, Args, ValueEnum};

use tlg::helper::get_tlg_type;
use tlg::{TlgReader, TlgType, TlgWriter};

/// A high-performance TLG image format conversion tool implemented in pure Rust
#[derive(Parser, Debug)]
#[command(name = "tlg-convert", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Encode an image to TLG (requires specifying the version with -v)
    Encode(EncodeArgs),

    /// Decode TLG to another image format (version is auto-detected)
    Decode(DecodeArgs),
}

#[derive(clap::Args, Debug)]
struct EncodeArgs {
    /// Input file (PNG/JPEG/BMP/TLG…)
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output file (.tlg)
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,

    /// TLG version: 5 or 6
    #[arg(short = 'v', long = "version", value_enum, default_value_t = TlgVersion::V6)]
    tlg_version: TlgVersion,
}

#[derive(Args, Debug)]
struct DecodeArgs {
    /// Input TLG file
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output file (PNG/JPEG/BMP…)
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum TlgVersion {
    #[value(name = "5")]
    V5,
    #[value(name = "6")]
    V6,
}

fn main()
{
    let args = Cli::parse();

    match args.command {
        Command::Encode(args) => encode(args),
        Command::Decode(args) => decode(args)
    }
}

fn encode(args: EncodeArgs)
{
    if !args.input.exists() || !args.input.is_file()
    {
        println!("Input file not found.");
        return
    }

    let mut input_file = BufReader::new(File::open(&args.input)
        .expect("Failed to open input file."));
    let tlg_type = get_tlg_type(&mut input_file)
        .expect("Failed to determine TLG type.");

    let raw = match tlg_type {
        Some(_) => {
            let (raw, _) = TlgReader::new(input_file).read_to_image()
                .expect("Failed to decode image.");
            raw
        }
        None => {
            ImageReader::new(input_file)
                .with_guessed_format().expect("The image format is not supported.")
                .decode().expect("Failed to decode image.")
        }
    };

    let mut output_file = File::create(args.output)
        .expect("Failed to create output file.");

    match args.tlg_version
    {
        TlgVersion::V5 => {
            let writer = TlgWriter::from_image(&raw, HashMap::new(), TlgType::Tlg5)
                .expect("Failed to create TLG5 writer.");
            writer.write_to(&mut output_file).expect("Failed to write TLG file.");
        }
        TlgVersion::V6 => {
            let writer = TlgWriter::from_image(&raw, HashMap::new(), TlgType::Tlg6)
                .expect("Failed to create TLG6 writer.");
            writer.write_to(&mut output_file).expect("Failed to write TLG file.");
        }
    }
}

fn decode(args: DecodeArgs)
{
    if !args.input.exists() || !args.input.is_file()
    {
        println!("Input file not found.");
        return
    }

    let mut input_file = BufReader::new(File::open(args.input)
        .expect("Failed to open input file."));
    let input_tlg_type = get_tlg_type(&mut input_file)
        .expect("Failed to determine TLG type.");

    if input_tlg_type.is_none()
    {
        println!("Input file is not a TLG file.");
        return
    }

    let output_format = get_format_for_path(&args.output);

    if output_format.is_none()
    {
        println!("未知的输出格式");
        return
    }
    let output_format = output_format.unwrap();

    let mut output_file = BufWriter::new(File::create(&args.output)
        .expect("Failed to create output file."));

    let (raw, _) = TlgReader::from_reader(input_file).read_to_image()
        .expect("Failed to decode tlg image.");

    raw.write_to(&mut output_file, output_format)
        .unwrap_or_else(|e| {
            panic!(
                "failed to write image to {} as {:?}: {e}",
                args.output.display(),
                output_format
            )
        });
}

fn get_format_for_path<P: AsRef<Path>>(path: P) -> Option<ImageFormat>
{
    let ext = path.as_ref().extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_ascii_lowercase().as_str() {
        "avif" => Some(ImageFormat::Avif),
        "bmp" => Some(ImageFormat::Bmp),
        "dds" => Some(ImageFormat::Dds),
        "ff" => Some(ImageFormat::Farbfeld),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "png" => Some(ImageFormat::Png),
        "qoi" => Some(ImageFormat::Qoi),
        "pnm" | "pbm" | "pgm" | "ppm" | "pam" => Some(ImageFormat::Pnm),
        "tga" => Some(ImageFormat::Tga),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        _ => None,
    }
}
