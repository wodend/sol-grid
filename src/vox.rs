use crate::{Grid, Voxel};
use std::collections::HashMap;

use std::io::Write;

pub fn encode(grid: Grid<Voxel>) -> std::io::Result<Vec<u8>> {
    // Calculate vox data
    let mut color_indices = HashMap::new();
    let mut index = 1;
    let mut xyzis = Vec::new();
    for (x, y, z, v) in grid.enumerate_cells() {
        let mut xyzi = [0; 4];
        xyzi[0] = x as u8;
        xyzi[1] = y as u8;
        xyzi[2] = z as u8;
        let rgba = v.as_rgba();
        match color_indices.get(rgba) {
            None => {
                color_indices.insert(rgba, index);
                xyzi[3] = index;
                index += 1;
            },
            Some(i) => {
                xyzi[3] = *i as u8;
            },
        }
        if rgba[3] > 0 {
            xyzis.push(xyzi);
        }
    }
    // Vox spec: https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt
    let mut bytes = Vec::new();
    bytes.write(b"VOX ")?;
    bytes.write(&u32::to_le_bytes(150))?;

    const INT_SIZE: u32 = 4;
    const ZERO: [u8; 4] = [0; 4];
    let size_chunk_size = INT_SIZE * 3;
    // TODO: Handle cases where voxel count exeeds u32 bounds
    let voxel_count = xyzis.len() as u32;
    let xyzi_chunk_size = INT_SIZE + (voxel_count * INT_SIZE);
    const PALETTE_COUNT: u32 = 256;
    let rgba_chunk_size = PALETTE_COUNT * INT_SIZE;
    let chunk_header_size = INT_SIZE * 3;
    let chunk_count = 3;
    let main_child_chunks_size = (chunk_header_size * chunk_count)
        + size_chunk_size
        + xyzi_chunk_size
        + rgba_chunk_size;
    bytes.write(b"MAIN")?;
    bytes.write(&ZERO)?; // MAIN has no content
    bytes.write(&u32::to_le_bytes(main_child_chunks_size))?;

    bytes.write(b"SIZE")?;
    bytes.write(&u32::to_le_bytes(size_chunk_size))?;
    bytes.write(&ZERO)?; // SIZE has no children
    bytes.write(&u32::to_le_bytes(grid.width()))?;
    bytes.write(&u32::to_le_bytes(grid.depth()))?;
    bytes.write(&u32::to_le_bytes(grid.height()))?;

    bytes.write(b"XYZI")?;
    bytes.write(&u32::to_le_bytes(xyzi_chunk_size))?;
    bytes.write(&ZERO)?; // XYZI has no children
    bytes.write(&u32::to_le_bytes(voxel_count))?;
    // TODO: Handle cases where xyzi exceeds u8 bounds
    for xyzi in &xyzis {
        bytes.write(xyzi)?;
    }

    bytes.write(b"RGBA")?;
    bytes.write(&u32::to_le_bytes(rgba_chunk_size))?;
    bytes.write(&ZERO)?; // RGBA has no children
    let mut palette = [[0; 4]; PALETTE_COUNT as usize];
    for (rgba, i) in color_indices {
        palette[i as usize - 1] = rgba.try_into().unwrap();
    }
    bytes.write(&palette.concat())?;
    Ok(bytes)
}