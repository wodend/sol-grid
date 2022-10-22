use std::marker::PhantomData;
use std::ops::Range;

pub trait Codec {
    const SIZE: u8;

    fn as_slice(&self) -> &[u8];
    fn from_slice(slice: &[u8]) -> &Self;
    fn from_slice_mut(slice: &mut [u8]) -> &mut Self;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voxel([u8; 16]);

impl Voxel {
    pub fn from_rgba(rgba: &[u8]) -> Voxel {
        *<Voxel>::from_slice(&[rgba, &[0; 12]].concat()[..])
    }

    pub fn as_rgba(&self) -> &[u8] {
        &self.0[0..4]
    }
}

impl Codec for Voxel {
    const SIZE: u8 = 16;

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

pub struct Grid<T> {
    width: u32,
    depth: u32,
    height: u32,
    data: Vec<u8>,
    _phantom: PhantomData<T>,
}
impl<T> Grid<T>
where
    T: Codec,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let voxel_red = Voxel([255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(voxel_red.as_rgba(), [255, 0, 0, 255]);
    }

    #[test]
    fn test_voxel_from_rgba() {
        let red = [255, 0, 0, 255];
        let value = Voxel::from_rgba(&red);
        assert_eq!(value, Voxel([255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn test_voxel_as_slice() {
        let array = [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let voxel = Voxel(array);
        assert_eq!(voxel.as_slice(), array);
    }

    #[test]
    fn test_voxel_from_slice() {
        let array = [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let voxel = <Voxel>::from_slice(&array);
        assert_eq!(*voxel, Voxel([255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn test_voxel_from_slice_mut() {
        let mut array = [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let voxel = <Voxel>::from_slice_mut(&mut array);
        voxel.0[1] = 255;
        assert_eq!(*voxel, Voxel([255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
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
}
