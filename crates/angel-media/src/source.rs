use {
    super::error,
    ffmpeg::{
        codec::{decoder::Video as VideoDecoder, Context as Codec},
        format::context::Input as Inner,
        media::Type as MediaKind,
        util::{format::Pixel as PixelFormat, frame::Video as VideoFrame},
    },
    image::{Delay, Frame, ImageResult, RgbaImage},
    std::{
        fmt,
        sync::{Arc, Mutex, PoisonError},
    },
};

pub struct MediaSource {
    pub(crate) inner: Arc<Mutex<Inner>>,
}

pub struct VideoSource {
    inner: Arc<Mutex<Inner>>,
    index: usize,
    decoder: VideoDecoder,
    is_eof: bool,
}

impl MediaSource {
    /// Try to find a video source from the media source.
    pub fn video(&self) -> ImageResult<VideoSource> {
        video_source(self)
    }
}

impl Iterator for VideoSource {
    type Item = ImageResult<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        next_frame(self).transpose()
    }
}

impl fmt::Debug for MediaSource {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("MediaSource").finish_non_exhaustive()
    }
}

impl fmt::Debug for VideoSource {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("VideoSource").finish_non_exhaustive()
    }
}

/// Try to find a video source from the media source.
pub fn video_source(media_source: &MediaSource) -> ImageResult<VideoSource> {
    let inner = Arc::clone(&media_source.inner);
    let (index, decoder) = {
        let source = inner.lock().unwrap_or_else(PoisonError::into_inner);

        let stream = source
            .streams()
            .best(MediaKind::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)
            .map_err(error::ffmpeg_into_image)?;

        let index = stream.index();
        let parameters = stream.parameters();
        let codec = Codec::from_parameters(parameters).map_err(error::ffmpeg_into_image)?;
        let decoder = codec.decoder().video().map_err(error::ffmpeg_into_image)?;

        (index, decoder)
    };

    tracing::debug!("obtained a video stream");

    Ok(VideoSource {
        inner,
        index,
        decoder,
        is_eof: false,
    })
}

/// Try to decode the next video frame from a video source.
pub fn next_frame(video_source: &mut VideoSource) -> ImageResult<Option<Frame>> {
    let VideoSource {
        inner,
        index,
        decoder,
        is_eof,
    } = video_source;

    if *is_eof {
        return Ok(None);
    }

    let mut source = inner.lock().unwrap_or_else(PoisonError::into_inner);

    for (stream, packet) in source.packets() {
        if stream.index() != *index {
            tracing::debug!("skip unknown index");

            continue;
        }

        tracing::debug!("send packet");

        decoder
            .send_packet(&packet)
            .map_err(error::ffmpeg_into_image)?;

        let mut frame = VideoFrame::empty();

        tracing::debug!("decode frame");

        decoder
            .receive_frame(&mut frame)
            .map_err(error::ffmpeg_into_image)?;

        if let Some(frame) = to_rgba(&frame)? {
            tracing::debug!("emit frame");

            return Ok(Some(frame));
        }
    }

    tracing::debug!("end of video");

    *is_eof = true;

    decoder.send_eof().map_err(error::ffmpeg_into_image)?;

    Ok(None)
}

/// Convert a [`VideoFrame`] to a [`Frame`].
pub fn to_rgba(source: &VideoFrame) -> ImageResult<Option<Frame>> {
    let mut rgba = VideoFrame::empty();

    source
        .converter(PixelFormat::RGBA)
        .map_err(error::ffmpeg_into_image)?
        .run(source, &mut rgba)
        .map_err(error::ffmpeg_into_image)?;

    if rgba.planes() > 0 {
        let width = rgba.width();
        let height = rgba.height();
        let data = rgba.data(0).to_vec();
        let image = RgbaImage::from_raw(width, height, data).ok_or_else(error::out_of_memory)?;

        //let delay = Delay::from_saturating_duration(Duration::from_secs())
        let delay = Delay::from_numer_denom_ms(10, 1);

        tracing::debug!("converted frame to rgba");

        Ok(Some(Frame::from_parts(image, 0, 0, delay)))
    } else {
        Ok(None)
    }
}
