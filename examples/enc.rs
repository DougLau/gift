use gift::block::*;
use gift::*;
use std::error::Error;
use std::fs::File;

fn main() -> Result<(), Box<Error>> {
    let mut f = File::create("test.gif")?;
    let g_tbl = ColorTableConfig::new(
        ColorTableExistence::Present,
        ColorTableOrdering::NotSorted,
        2,
    );
    let l_tbl = ColorTableConfig::default();
    let colors = [0, 0, 0, 255, 255, 255];
    let mut image = ImageData::new(16);
    image.add_data(&[
        1, 0, 0, 1,
        0, 1, 1, 0,
        0, 1, 1, 0,
        1, 0, 0, 1,
    ]);
    let mut enc = Encoder::new(&mut f);
    enc.encode(&Header::with_version(*b"89a").into())?;
    enc.encode(
        &LogicalScreenDesc::default()
            .with_screen_width(4)
            .with_screen_height(4)
            .with_color_table_config(&g_tbl)
            .with_background_color_idx(1)
            .into(),
    )?;
    enc.encode(&GlobalColorTable::with_colors(&colors).into())?;
    enc.encode(
        &ImageDesc::default()
            .with_width(4)
            .with_height(4)
            .with_color_table_config(&l_tbl)
            .into(),
    )?;
    enc.encode(&image.into())?;
    enc.encode(&Trailer::default().into())?;
    Ok(())
}
