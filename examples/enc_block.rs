// Block encoding example
use gift::{block::*, Encoder};
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn Error>> {
    let mut f = BufWriter::new(File::create("test.gif")?);
    let g_tbl = ColorTableConfig::new(
        ColorTableExistence::Present,
        ColorTableOrdering::NotSorted,
        2,
    );
    let l_tbl = ColorTableConfig::default();
    let colors = [0, 0, 0, 255, 255, 255];
    let mut image = ImageData::new(16);
    #[rustfmt::skip]
    image.data_mut().extend(&[
        1, 0, 0, 1,
        0, 1, 1, 0,
        0, 1, 1, 0,
        1, 0, 0, 1,
    ]);
    let mut blocks = Encoder::new(&mut f).into_block_enc();
    blocks.encode(Header::with_version(*b"89a"))?;
    blocks.encode(
        LogicalScreenDesc::default()
            .with_screen_width(4)
            .with_screen_height(4)
            .with_color_table_config(g_tbl)
            .with_background_color_idx(1),
    )?;
    blocks.encode(GlobalColorTable::with_colors(&colors))?;
    blocks.encode(
        ImageDesc::default()
            .with_width(4)
            .with_height(4)
            .with_color_table_config(l_tbl),
    )?;
    blocks.encode(image)?;
    blocks.encode(Trailer::default())?;
    Ok(())
}
