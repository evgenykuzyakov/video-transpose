use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{input, Pixel};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::util::frame::video::Video;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_video> <output_video>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Loading video: {}", input_path);
    println!("This will transpose X (horizontal) and T (time) axes");
    println!("Original: X×Y pixels, T frames → Output: T×Y pixels, X frames\n");

    // Open input video
    let mut ictx = input(&Path::new(input_path))?;

    // Find video stream and get parameters
    let input_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or("No video stream found")?;
    let video_stream_index = input_stream.index();

    // Get stream parameters before borrowing mutably
    let stream_params = input_stream.parameters();
    let fps = input_stream.avg_frame_rate();

    // Get decoder
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(stream_params)?;
    let mut decoder = context_decoder.decoder().video()?;

    let width = decoder.width() as usize;
    let height = decoder.height() as usize;
    let decoder_format = decoder.format();

    println!("Input video info:");
    println!("  Resolution: {}×{}", width, height);
    println!(
        "  Frame rate: {}/{} fps",
        fps.numerator(),
        fps.denominator()
    );

    // First pass: decode all frames into memory
    println!("\n[1/2] Decoding all frames...");
    let mut frames = Vec::new();

    // Create scaler to RGB24 for easier manipulation
    let mut scaler = Context::get(
        decoder_format,
        width as u32,
        height as u32,
        Pixel::RGB24,
        width as u32,
        height as u32,
        Flags::BILINEAR,
    )?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {pos} frames decoded")
            .unwrap(),
    );

    // Decode all frames
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            receive_and_process_frames(&mut decoder, &mut scaler, &mut frames, &pb)?;
        }
    }

    // Flush decoder
    decoder.send_eof()?;
    receive_and_process_frames(&mut decoder, &mut scaler, &mut frames, &pb)?;

    pb.finish_with_message(format!("{} frames decoded", frames.len()));

    let num_frames = frames.len();
    if num_frames == 0 {
        return Err("No frames decoded".into());
    }

    println!("\n[2/2] Transposing axes and encoding...");
    println!(
        "  Output will be: {}×{} pixels, {} frames",
        num_frames, height, width
    );

    // Create output video
    transpose_and_save(frames, width, height, num_frames, output_path, fps)?;

    println!("\n✓ Video transposition complete!");
    println!("  Output saved to: {}", output_path);

    Ok(())
}

fn receive_and_process_frames(
    decoder: &mut ffmpeg::decoder::Video,
    scaler: &mut Context,
    frames: &mut Vec<Vec<u8>>,
    pb: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoded = Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        let mut rgb_frame = Video::empty();
        scaler.run(&decoded, &mut rgb_frame)?;

        // Copy frame data
        let data = rgb_frame.data(0).to_vec();
        frames.push(data);

        pb.inc(1);
    }
    Ok(())
}

fn transpose_and_save(
    input_frames: Vec<Vec<u8>>,
    orig_width: usize,
    orig_height: usize,
    num_frames: usize,
    output_path: &str,
    fps: ffmpeg::Rational,
) -> Result<(), Box<dyn std::error::Error>> {
    // Output dimensions: T×Y pixels, X frames
    let new_width_raw = num_frames;
    let new_height = orig_height;
    let new_num_frames = orig_width;

    // H.264 requires even dimensions, pad if needed
    let new_width = if new_width_raw % 2 == 0 {
        new_width_raw
    } else {
        new_width_raw + 1
    };

    let padded = new_width != new_width_raw;

    if padded {
        println!(
            "  Note: Padding width from {} to {} (H.264 requires even dimensions)",
            new_width_raw, new_width
        );
    }

    let pb = ProgressBar::new(new_num_frames as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} frames",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    // Setup FFmpeg output
    let mut octx = ffmpeg::format::output(&output_path)?;

    // Get format flags before creating encoder
    let global_header = octx
        .format()
        .flags()
        .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);

    // Find H264 encoder
    let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264).ok_or("H264 encoder not found")?;

    // Create and configure encoder context FIRST
    let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
        .encoder()
        .video()?;

    encoder.set_width(new_width as u32);
    encoder.set_height(new_height as u32);
    encoder.set_format(Pixel::YUV420P);

    // Time base should be inverse of frame rate
    // For 29.97 fps (30000/1001), time_base should be 1001/30000
    encoder.set_time_base(ffmpeg::Rational(fps.denominator(), fps.numerator()));
    encoder.set_frame_rate(Some(fps));
    encoder.set_max_b_frames(0);

    if global_header {
        encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
    }

    // Open encoder
    let mut encoder = encoder.open_as(codec)?;
    let encoder_time_base = encoder.time_base();

    // NOW add stream and copy parameters
    let mut ostream = octx.add_stream(codec)?;
    let stream_index = ostream.index();

    // Copy encoder parameters to stream
    ostream.set_parameters(&encoder);
    ostream.set_time_base(ffmpeg::Rational(fps.denominator(), fps.numerator()));
    ostream.set_avg_frame_rate(fps);

    println!(
        "  Input FPS: {}/{} ({:.2} fps)",
        fps.numerator(),
        fps.denominator(),
        fps.numerator() as f64 / fps.denominator() as f64
    );
    println!(
        "  Encoder time base: {}/{}",
        encoder_time_base.numerator(),
        encoder_time_base.denominator()
    );
    println!(
        "  Stream time base before header: {}/{}",
        ostream.time_base().numerator(),
        ostream.time_base().denominator()
    );

    // Create scaler
    let mut scaler = Context::get(
        Pixel::RGB24,
        new_width as u32,
        new_height as u32,
        Pixel::YUV420P,
        new_width as u32,
        new_height as u32,
        Flags::BILINEAR,
    )?;

    // Write header - this may change the time base!
    octx.write_header()?;

    // Get the ACTUAL time base that the muxer is using after write_header
    let actual_stream_time_base = octx
        .stream(stream_index)
        .ok_or("Stream not found")?
        .time_base();

    println!(
        "  Stream time base AFTER header: {}/{}",
        actual_stream_time_base.numerator(),
        actual_stream_time_base.denominator()
    );

    // Calculate PTS increment for desired frame rate
    // For 29.97 fps (30000/1001) with time_base 1/30000:
    // pts_increment = (30000 * 1001) / 30000 = 1001
    let pts_increment = (actual_stream_time_base.denominator() as i64 * fps.denominator() as i64)
        / fps.numerator() as i64;
    println!("  PTS increment per frame: {}", pts_increment);

    // Process each output frame
    let mut current_pts: i64 = 0;
    for x in 0..new_num_frames {
        // Create transposed frame: new_width × new_height
        let mut transposed_data = vec![0u8; new_width * new_height * 3];

        // For each pixel in the output frame
        for y in 0..new_height {
            for t in 0..new_width_raw {
                // Source: frame t, position (x, y)
                // Destination: frame x, position (t, y)
                let src_offset = (y * orig_width + x) * 3;
                let dst_offset = (y * new_width + t) * 3;

                // Copy RGB values
                transposed_data[dst_offset] = input_frames[t][src_offset];
                transposed_data[dst_offset + 1] = input_frames[t][src_offset + 1];
                transposed_data[dst_offset + 2] = input_frames[t][src_offset + 2];
            }

            // If padded, duplicate the last column
            if padded {
                let last_src_offset = (y * new_width + new_width_raw - 1) * 3;
                let pad_dst_offset = (y * new_width + new_width_raw) * 3;

                transposed_data[pad_dst_offset] = transposed_data[last_src_offset];
                transposed_data[pad_dst_offset + 1] = transposed_data[last_src_offset + 1];
                transposed_data[pad_dst_offset + 2] = transposed_data[last_src_offset + 2];
            }
        }

        // Create frame from transposed data
        let mut rgb_frame = Video::new(Pixel::RGB24, new_width as u32, new_height as u32);

        // Get the stride (linesize) for the frame
        let linesize = rgb_frame.stride(0);
        let frame_data = rgb_frame.data_mut(0);

        // Copy row by row, respecting the stride
        for y in 0..new_height {
            let src_start = y * new_width * 3;
            let src_end = src_start + new_width * 3;
            let dst_start = y * linesize;
            let dst_end = dst_start + new_width * 3;

            frame_data[dst_start..dst_end].copy_from_slice(&transposed_data[src_start..src_end]);
        }

        // Convert to YUV420P
        let mut yuv_frame = Video::empty();
        scaler.run(&rgb_frame, &mut yuv_frame)?;

        // Set PTS in encoder time base
        yuv_frame.set_pts(Some(x as i64));

        // Encode frame
        encoder.send_frame(&yuv_frame)?;

        // Receive and write packets with proper PTS scaling
        receive_and_write_packets_with_pts(
            &mut encoder,
            &mut octx,
            stream_index,
            encoder_time_base,
            actual_stream_time_base,
            &mut current_pts,
            pts_increment,
        )?;

        pb.inc(1);
    }

    // Flush encoder
    encoder.send_eof()?;
    receive_and_write_packets_with_pts(
        &mut encoder,
        &mut octx,
        stream_index,
        encoder_time_base,
        actual_stream_time_base,
        &mut current_pts,
        pts_increment,
    )?;

    // Write trailer
    octx.write_trailer()?;
    pb.finish_with_message("Encoding complete");

    Ok(())
}

fn receive_and_write_packets_with_pts(
    encoder: &mut ffmpeg::encoder::video::Video,
    octx: &mut ffmpeg::format::context::Output,
    stream_index: usize,
    encoder_time_base: ffmpeg::Rational,
    stream_time_base: ffmpeg::Rational,
    current_pts: &mut i64,
    pts_increment: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoded_packet = ffmpeg::Packet::empty();

    while encoder.receive_packet(&mut encoded_packet).is_ok() {
        encoded_packet.set_stream(stream_index);

        // Rescale from encoder time base to stream time base
        encoded_packet.rescale_ts(encoder_time_base, stream_time_base);

        // Override PTS/DTS with our calculated values for correct frame rate
        encoded_packet.set_pts(Some(*current_pts));
        encoded_packet.set_dts(Some(*current_pts));

        *current_pts += pts_increment;

        encoded_packet.write_interleaved(octx)?;
    }
    Ok(())
}
