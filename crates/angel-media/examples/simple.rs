use {
    angel_media::{Context, MediaFormat, Reader},
    image::ImageResult,
    std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
    },
};

fn main() -> ImageResult<()> {
    let mut sample = File::open("examples/sample.mp4")?;
    let mut buf = [0; 4096];

    sample.read(&mut buf)?;
    sample.seek(SeekFrom::Start(0))?;

    let format = MediaFormat::guess(&buf)?;

    println!("Format: {format:?}");

    let mut context = Context::new()?;

    println!("Context: {context:?}");

    context.set_reader(Reader::new(sample))?;

    println!("Context: {context:?}");

    let media_source = context.decode(format)?;

    println!("Media source: {media_source:?}");

    let video_source = media_source.video()?;

    println!("Video source: {video_source:?}");

    for (index, frame) in video_source.enumerate() {
        let frame = frame?;

        println!("Decoded frame {index}: {:?}", frame.delay());
    }

    Ok(())
}
