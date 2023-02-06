pub mod vox;

use std::marker::PhantomData;
use std::ops::Range;
use std::slice::{ChunksExact, ChunksExactMut};

pub trait Codec {
    const SIZE: u8;

    fn as_slice(&self) -> &[u8];
    fn from_slice(slice: &[u8]) -> &Self;
    fn from_slice_mut(slice: &mut [u8]) -> &mut Self;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voxel([u8; 8]);

impl Voxel {
    pub fn from_rgba(rgba: &[u8]) -> Voxel {
        *<Voxel>::from_slice(&[rgba, &[0; 4]].concat()[..])
    }

    pub fn as_rgba(&self) -> &[u8] {
        &self.0[0..4]
    }
}

impl Codec for Voxel {
    const SIZE: u8 = 8;

    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn from_slice(slice: &[u8]) -> &Voxel {
        assert_eq!(slice.len(), Self::SIZE as usize);
        unsafe { &*(slice.as_ptr() as *const Voxel) }
    }

    fn from_slice_mut(slice: &mut [u8]) -> &mut Voxel {
        assert_eq!(slice.len(), Self::SIZE as usize);
        unsafe { &mut *(slice.as_mut_ptr() as *mut Voxel) }
    }
}

impl Codec for u32 {
    const SIZE: u8 = (u32::BITS / 8) as u8;

    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        unsafe { std::mem::transmute::<&u32, &[u8; Self::SIZE as usize]>(self) }
    }

    fn from_slice(slice: &[u8]) -> &u32 {
        assert_eq!(slice.len(), Self::SIZE as usize);
        unsafe { &*(slice.as_ptr() as *const u32) }
    }

    fn from_slice_mut(slice: &mut [u8]) -> &mut u32 {
        assert_eq!(slice.len(), Self::SIZE as usize);
        unsafe { &mut *(slice.as_mut_ptr() as *mut u32) }
    }
}

#[derive(Debug)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

pub struct Grid<T> {
    width: u32,
    depth: u32,
    height: u32,
    data: Vec<u8>,
    _phantom: PhantomData<T>,
}
impl<T> Grid<T>
where
    T: Codec + Copy,
{
    pub fn new(width: u32, depth: u32, height: u32) -> Grid<T> {
        match Self::len(width, depth, height) {
            None => panic!("Grid len overflows usize"),
            Some(len) => {
                Grid {
                    width: width,
                    depth: depth,
                    height: height,
                    data: vec![0; len],
                    _phantom: PhantomData,
                }
            },
        }
    }

    fn len(width: u32, depth: u32, height: u32) -> Option<usize> {
        Some(<T>::SIZE as usize)
            .and_then(|size| size.checked_mul(width as usize))
            .and_then(|size| size.checked_mul(depth as usize))
            .and_then(|size| size.checked_mul(height as usize))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32, z: u32) -> &T {
        match self.indices(x, y, z) {
            None => panic!(
                "Grid index {:?} out of bounds {:?}",
                (x, y, z),
                (self.width, self.depth, self.height)
            ),
            Some(indices) => <T>::from_slice(&self.data[indices]),
        }
    }

    pub fn get_mut(&mut self, x: u32, y: u32, z: u32) -> &mut T {
        match self.indices(x, y, z) {
            None => panic!(
                "Grid index {:?} out of bounds {:?}",
                (x, y, z),
                (self.width, self.depth, self.height)
            ),
            Some(indices) => <T>::from_slice_mut(&mut self.data[indices]),
        }
    }

    #[inline(always)]
    fn indices(&self, x: u32, y: u32, z: u32) -> Option<Range<usize>> {
        if x >= self.width || y >= self.depth || z >= self.height {
            return None;
        }
        Some(self.indices_unchecked(x, y, z))
    }

    #[inline(always)]
    fn indices_unchecked(&self, x: u32, y: u32, z: u32) -> Range<usize> {
        let unsized_index = x + (y * self.width) + (z * self.width * self.depth);
        let min_index = unsized_index as usize * <T>::SIZE as usize;
        min_index..min_index + <T>::SIZE as usize
    }

    pub fn enumerate_cells(&self) -> EnumerateCells<T> {
        EnumerateCells {
            chunks: self
                .data
                .chunks_exact(<T>::SIZE as usize),
            x: 0,
            y: 0,
            z: 0,
            width: self.width,
            depth: self.depth,
            _phantom: PhantomData,
        }
    }

    pub fn enumerate_cells_mut(&mut self) -> EnumerateCellsMut<T> {
        EnumerateCellsMut {
            chunks: self
                .data
                .chunks_exact_mut(<T>::SIZE as usize),
            x: 0,
            y: 0,
            z: 0,
            width: self.width,
            depth: self.depth,
            _phantom: PhantomData,
        }
    }


    pub fn cell_count(&self) -> usize {
        self.width as usize * self.depth as usize * self.height as usize
    }

    pub fn rotated_z(&self, rotation: &Rotation) -> Grid<T> {
        let width = self.width();
        let depth = self.depth();
        let height = self.height();
        // TODO: Figure out how to do it for rectangular prisms
        assert!(width == depth && width == height);
        let mut output = Grid::new(width, depth, height);
        let r = match rotation {
            Rotation::R0 => [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
            Rotation::R90 => [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
            Rotation::R180 => [[-1, 0, 0], [0, -1, 0], [0, 0, 1]],
            Rotation::R270 => [[0, 1, 0], [-1, 0, 0], [0, 0, 1]],
        };
        let x_offset = width as i64 / 2;
        let y_offset = depth as i64 / 2;
        let z_offset = height as i64 / 2;
        let x_even_correction = match rotation {
            Rotation::R90 | Rotation::R180 =>  if width % 2 == 0 { 1 } else { 0 }
            _ => if depth > width { 1 } else { 0 },
        };
        let y_even_correction = match rotation {
            Rotation::R180 | Rotation::R270 =>  if depth % 2 == 0 { 1 } else { 0 }
            _ => 0,
        };
        for (gx, gy, gz, t) in self.enumerate_cells() {
            let x = gx as i64 - x_offset;
            let y = gy as i64 - y_offset;
            let z = gz as i64 - z_offset;
            let rx = r[0][0] * x + r[0][1] * y + r[0][2] * z + x_offset - x_even_correction;
            let ry = r[1][0] * x + r[1][1] * y + r[1][2] * z + y_offset - y_even_correction;
            let rz = r[2][0] * x + r[2][1] * y + r[2][2] * z + z_offset;
            *output.get_mut(rx as u32, ry as u32, rz as u32) = *t;
        }
        output
    }
}

pub struct EnumerateCells<'a, T> {
    chunks: ChunksExact<'a, u8>,
    x: u32,
    y: u32,
    z: u32,
    width: u32,
    depth: u32,
    _phantom: PhantomData<T>,
}

impl<'a, T> Iterator for EnumerateCells<'a, T>
where
    T: Codec + 'a,
{
    type Item = (u32, u32, u32, &'a T);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= self.depth {
            self.y = 0;
            self.z += 1;
        }
        let (x, y, z) = (self.x, self.y, self.z);
        self.x += 1;
        self.chunks.next().map(|t| (x, y, z, <T>::from_slice(t)))
    }
}


pub struct EnumerateCellsMut<'a, T> {
    chunks: ChunksExactMut<'a, u8>,
    x: u32,
    y: u32,
    z: u32,
    width: u32,
    depth: u32,
    _phantom: PhantomData<T>,
}

impl<'a, T> Iterator for EnumerateCellsMut<'a, T>
where
    T: Codec + 'a,
{
    type Item = (u32, u32, u32, &'a mut T);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= self.depth {
            self.y = 0;
            self.z += 1;
        }
        let (x, y, z) = (self.x, self.y, self.z);
        self.x += 1;
        self.chunks.next().map(|t| (x, y, z, <T>::from_slice_mut(t)))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const RED_SLICE: [u8; 8] = [255, 0, 0, 255, 0, 0, 0, 0];
    const RED_VOXEL: Voxel = Voxel(RED_SLICE);

    #[test]
    fn test_u32_as_slice() {
        let value = 1.as_slice();
        let slice = [1, 0, 0, 0];
        assert_eq!(*value, slice);
    }

    #[test]
    fn test_u32_from_slice() {
        let slice = [1, 0, 0, 0];
        let value = <u32>::from_slice(&slice);
        assert_eq!(*value, 1);
    }

    #[test]
    fn test_u32_from_slice_mut() {
        let mut slice = [1, 0, 0, 0];
        let value = <u32>::from_slice_mut(&mut slice);
        *value += 1;
        assert_eq!(*value, 2);
        assert_eq!(slice, [2, 0, 0, 0]);
    }

    #[test]
    fn test_voxel_as_rgba() {
        assert_eq!(RED_VOXEL.as_rgba(), RED);
    }

    #[test]
    fn test_voxel_from_rgba() {
        assert_eq!(Voxel::from_rgba(&RED), RED_VOXEL);
    }

    #[test]
    fn test_voxel_as_slice() {
        assert_eq!(RED_VOXEL.as_slice(), RED_SLICE);
    }

    #[test]
    fn test_voxel_from_slice() {
        assert_eq!(*<Voxel>::from_slice(&RED_SLICE), RED_VOXEL);
    }

    #[test]
    fn test_voxel_from_slice_mut() {
        let mut array = [0u8; 8];
        array.copy_from_slice(&RED_SLICE);
        let voxel = <Voxel>::from_slice_mut(&mut array);
        voxel.0[1] = 255;
        assert_eq!(*voxel, Voxel([255, 255, 0, 255, 0, 0, 0, 0]));
    }

    #[test]
    fn test_grid() {
        let grid_width = 3;
        let grid_depth = 3;
        let grid_height = 3;
        let mut grid = Grid::new(grid_width, grid_depth, grid_height);

        let mut order = 0;
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    order += 1;
                    *grid.get_mut(x, y, z) = order;
                }
            }
        }
        order = 0;
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    order += 1;
                    let value = grid.get(x, y, z);
                    assert_eq!(*value, order);
                }
            }
        }
    }

    #[test]
    fn test_grid_voxel() {
        let grid_width = 3;
        let grid_depth = 3;
        let grid_height = 3;
        let mut grid = Grid::new(grid_width, grid_depth, grid_height);

        let red = [255, 0, 0, 255];
        let voxel = Voxel::from_rgba(&red);
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    *grid.get_mut(x, y, z) = voxel;
                }
            }
        }
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    let value = grid.get(x, y, z);
                    assert_eq!(value.as_rgba(), red);
                }
            }
        }
    }

    #[test]
    fn test_grid_enumerate_cells() {
        let grid_width = 3;
        let grid_depth = 3; 
        let grid_height = 3;
        let mut grid = Grid::new(grid_width, grid_depth, grid_height);
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    let color = [x as u8, y as u8, z as u8, 255];
                    let voxel = Voxel::from_rgba(&color);
                    *grid.get_mut(x, y, z) = voxel;
                }
            }
        }
        for (x, y, z, v) in grid.enumerate_cells() {
            let rgba = v.as_rgba();
            assert_eq!(x as u8, rgba[0]);
            assert_eq!(y as u8, rgba[1]);
            assert_eq!(z as u8, rgba[2]);
        }
    }

    #[test]
    fn test_grid_cell_count() {
        let grid_width = 3;
        let grid_depth = 3; 
        let grid_height = 3;
        let grid = Grid::<u32>::new(grid_width, grid_depth, grid_height);
        assert_eq!(grid.cell_count(), 27);
    }

    #[test]
    fn test_vox_write() {
        let grid_width = 3;
        let grid_depth = 3; 
        let grid_height = 3;
        let mut grid = Grid::new(grid_width, grid_depth, grid_height);
        let black = [0, 0, 0, 255];
        let white = [255, 255, 255, 255];
        let voxel_black = Voxel::from_rgba(&black);
        let voxel_white = Voxel::from_rgba(&white);
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    if z == 0 {
                        *grid.get_mut(x, y, z) = voxel_black;
                    } else {
                        *grid.get_mut(x, y, z) = voxel_white;
                    }
                }
            }
        }
        let bytes = vox::encode(&grid).unwrap();
        fs::write("test.vox", &bytes).unwrap();
    }

    #[test]
    fn test_vox_write_transparent_voxel() {
        let grid_width = 3;
        let grid_depth = 3;
        let grid_height = 3;
        let mut grid = Grid::new(grid_width, grid_depth, grid_height);

        let blue = [0, 0, 255, 0];
        let voxel = Voxel::from_rgba(&blue);
        for x in 0..grid_width {
            for y in 0..grid_depth {
                for z in 0..grid_height {
                    *grid.get_mut(x, y, z) = voxel;
                }
            }
        }
        let bytes = vox::encode(&grid).unwrap();
        fs::write("test_transparent.vox", &bytes).unwrap();
    }

    fn gen_test_road_edge() -> Grid<Voxel> {
        let width = 3;
        let depth = 3;
        let height = 3;
        let mut grid = Grid::new(width, depth, height);
        let grey = Voxel::from_rgba(&[108, 108, 127, 255]);
        let green = Voxel::from_rgba(&[90, 120, 20, 255]);
        let brown = Voxel::from_rgba(&[120, 80, 50, 255]);
        for x in 0..width {
            for y in 0..depth {
                for z in 0..height {
                    if z == height - 1 && x <= width / 2 {
                        *grid.get_mut(x, y, z) = grey;
                    } else if z == height - 1 {
                        *grid.get_mut(x, y, z) = green;
                    } else {
                        *grid.get_mut(x, y, z) = brown;
                    }
                }
            }
        }
        grid
    }

    #[test]
    fn test_voxel_rotated_z_0() {
        let grid = gen_test_road_edge();
        let rotated = grid.rotated_z(&Rotation::R0);
        let bytes = vox::encode(&rotated).unwrap();
        fs::write("test_road_rotated_z_0.vox", &bytes).unwrap();
    }

    #[test]
    fn test_voxel_rotated_z_90() {
        let grid = gen_test_road_edge();
        let rotated = grid.rotated_z(&Rotation::R90);
        let bytes = vox::encode(&rotated).unwrap();
        fs::write("test_road_rotated_z_90.vox", &bytes).unwrap();
    }

    #[test]
    fn test_voxel_rotated_z_180() {
        let grid = gen_test_road_edge();
        let rotated = grid.rotated_z(&Rotation::R180);
        let bytes = vox::encode(&rotated).unwrap();
        fs::write("test_road_rotated_z_180.vox", &bytes).unwrap();
    }

    #[test]
    fn test_voxel_rotated_z_270() {
        let grid = gen_test_road_edge();
        let rotated = grid.rotated_z(&Rotation::R270);
        let bytes = vox::encode(&rotated).unwrap();
        fs::write("test_road_rotated_z_270.vox", &bytes).unwrap();
    }
}
