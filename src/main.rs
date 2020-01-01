// main.rs      gift command
//
// Copyright (c) 2019  Douglas Lau
//
#![forbid(unsafe_code)]

use gift::block::{DisposalMethod, Frame};
use gift::Decoder;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> Result<(), Box<Error>> {
    let mut out = StandardStream::stdout(ColorChoice::Always);
    let mut red = ColorSpec::new();
    red.set_fg(Some(Color::Red)).set_intense(true);
    // FIXME: add subcommands: unwrap, wrap, peek, show
    if let Some(cmd) = env::args().nth(0) {
        if let Some(path) = env::args().nth(1) {
            show(&mut out, path)?;
        } else {
            out.set_color(&red)?;
            writeln!(out, "usage: {} [filename]", cmd)?;
        }
    } else {
        out.set_color(&red)?;
        writeln!(out, "environment failure!")?;
    }
    out.reset()?;
    Ok(())
}

fn show(out: &mut StandardStream, path: String) -> Result<(), Box<Error>> {
    let mut magenta = ColorSpec::new();
    magenta.set_fg(Some(Color::Magenta));
    let mut red = ColorSpec::new();
    red.set_fg(Some(Color::Red)).set_intense(true);
    let mut yellow = ColorSpec::new();
    yellow.set_fg(Some(Color::Yellow)).set_intense(true);
    let mut cyan = ColorSpec::new();
    cyan.set_fg(Some(Color::Cyan)).set_intense(true);
    let mut bold = ColorSpec::new();
    bold.set_fg(Some(Color::White))
        .set_intense(true)
        .set_bold(true);
    let f = File::open(&path)?;
    let mut frame_dec = Decoder::new(f).into_frame_decoder();
    let preamble = if let Some(p) = frame_dec.preamble()? {
        p
    } else {
        out.set_color(&red)?;
        writeln!(out, "no preamble!")?;
        return Ok(());
    };
    let mut frames = vec![];
    for f in frame_dec {
        frames.push(f?);
    }
    let frame_digits = digits(frames.len()).max(3);
    let width = preamble.screen_width();
    let height = preamble.screen_height();
    let size_digits = 4.max(1 + digits(width) + digits(height));
    let gif = String::from_utf8_lossy(&preamble.header.version()).to_string();
    let mut comments = vec![];
    for cmt in preamble.comments {
        for c in cmt.comments() {
            for l in String::from_utf8_lossy(c).split("\n") {
                let l = l.trim();
                if l.len() > 0 {
                    comments.push(l.to_string());
                }
            }
        }
    }
    out.set_color(&magenta)?;
    writeln!(out, "{}", path)?;
    out.set_color(&bold)?;
    write!(out, "GIF{}, frames: {}", gif, frames.len())?;
    if let Some(ap) = preamble.loop_count_ext {
        if let Some(c) = ap.loop_count() {
            write!(out, ", repeat: ")?;
            if c == 0 {
                write!(out, "∞")?;
            } else {
                write!(out, "{}", c)?;
            }
        }
    }
    if comments.len() > 0 {
        out.set_color(&cyan)?;
        for c in comments {
            writeln!(out, "  # {}", c)?;
        }
    } else {
        writeln!(out)?;
    }
    out.set_color(&yellow)?;
    write!(out, " {:>w$}", "Fr#", w = frame_digits)?;
    write!(out, "  Delay Disp")?;
    write!(out, " {:>w$}", "Size", w = size_digits)?;
    write!(out, " {:>w$}", "X,Y", w = size_digits)?;
    writeln!(out, " Clrs Trn")?;
    let global_clr = preamble.logical_screen_desc.color_table_config().len();
    for (n, f) in frames.into_iter().enumerate() {
        write_frame(
            &f,
            out,
            width,
            height,
            global_clr,
            n,
            frame_digits,
            size_digits,
        )?;
    }
    Ok(())
}

fn write_frame(
    frame: &Frame,
    out: &mut StandardStream,
    width: u16,
    height: u16,
    global_clr: usize,
    number: usize,
    frame_digits: usize,
    size_digits: usize,
) -> Result<(), Box<Error>> {
    let mut dflt = ColorSpec::new();
    dflt.set_fg(Some(Color::White));
    let mut bold = ColorSpec::new();
    bold.set_fg(Some(Color::White))
        .set_intense(true)
        .set_bold(true);
    let mut red = ColorSpec::new();
    red.set_fg(Some(Color::Red)).set_intense(true);
    out.set_color(&dflt)?;
    write!(
        out,
        "{}",
        if frame.image_desc.interlaced() {
            'i'
        } else {
            ' '
        }
    )?;
    out.set_color(&bold)?;
    write!(out, "{:>w$}", number, w = frame_digits)?;
    let d = if let Some(gc) = &frame.graphic_control_ext {
        gc.delay_time_cs()
    } else {
        0
    };
    if d == 0 {
        out.set_color(&dflt)?;
    }
    write!(out, " {:6.2}", d as f32 / 100f32)?;
    let d = if let Some(gc) = &frame.graphic_control_ext {
        match gc.disposal_method() {
            DisposalMethod::NoAction => "none",
            DisposalMethod::Keep => "keep",
            DisposalMethod::Background => "bg",
            DisposalMethod::Previous => "prev",
            _ => "res",
        }
    } else {
        "-"
    };
    out.set_color(match d {
        "none" | "-" => &dflt,
        "res" => &red,
        _ => &bold,
    })?;
    write!(out, " {:>4}", d)?;
    if width == frame.image_desc.width() && height == frame.image_desc.height()
    {
        out.set_color(&dflt)?;
    } else {
        out.set_color(&bold)?;
    }
    write!(
        out,
        " {:>w$}",
        &format!("{}x{}", frame.image_desc.width(), frame.image_desc.height()),
        w = size_digits
    )?;
    if frame.image_desc.left() == 0 && frame.image_desc.top() == 0 {
        out.set_color(&dflt)?;
    } else {
        out.set_color(&bold)?;
    }
    write!(
        out,
        " {:>w$}",
        &format!("{},{}", frame.image_desc.left(), frame.image_desc.top()),
        w = size_digits
    )?;
    let c = frame.image_desc.color_table_config().len();
    if c > 0 {
        out.set_color(&bold)?;
        write!(out, "  {:3}", c)?;
    } else {
        out.set_color(&dflt)?;
        write!(out, " {:3}g", global_clr)?;
    }
    let tc = if let Some(gc) = &frame.graphic_control_ext {
        if let Some(tc) = gc.transparent_color() {
            format!("{}", tc)
        } else {
            "-".to_string()
        }
    } else {
        "-".to_string()
    };
    if tc == "-" {
        out.set_color(&dflt)?;
    } else {
        out.set_color(&bold)?;
    }
    writeln!(out, " {:>3}", tc)?;
    Ok(())
}

fn digits<T: Into<usize>>(v: T) -> usize {
    let v = v.into();
    match v {
        0...9 => 1,
        10...99 => 2,
        100...999 => 3,
        1000...9999 => 4,
        _ => 5,
    }
}
