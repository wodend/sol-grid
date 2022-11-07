use sol_grid::{vox, Grid, Voxel};

use std::fs::write;

fn main() {
    // Grid orientation
    // Increasing x moves right
    // Increasing y moves away 
    // Increasing z moves up
    let grid_width = 3; // x bound
    let grid_depth = 3; // y bound
    let grid_height = 3; // z bound
    let mut grid = Grid::new(grid_width, grid_depth, grid_height);

    println!("Building example {:?} Grid<Voxel>", (grid_width, grid_depth, grid_height));
    // Voxels can be created from rgba values
    let black = [0, 0, 0, 255];
    let white = [255, 255, 255, 255];
    let voxel_black = Voxel::from_rgba(&black);
    let voxel_white = Voxel::from_rgba(&white);
    // Create a voxel grid with first vertical layer black and the rest white
    for x in 0..grid_width {
        for y in 0..grid_depth {
            for z in 0..grid_height {
                // Mutate references to voxels in the grid
                if z == 0 {
                    *grid.get_mut(x, y, z) = voxel_black;
                } else {
                    *grid.get_mut(x, y, z) = voxel_white;
                }
            }
        }
    }
    for x in 0..grid_width {
        for y in 0..grid_depth {
            for z in 0..grid_height {
                // Get references to voxels in the grid
                let voxel = grid.get(x, y, z);
                // Voxels can be cast as rgba values
                let color = voxel.as_rgba();
                if z == 0 {
                    assert_eq!(color, black);
                } else {
                    assert_eq!(color, white);
                }
                println!("Grid index {:?} voxel color {:?}", (x, y, z), color);
            }
        }
    }
    // Supports encoding a Grid<Voxel> as MagicaVoxel .vox format
    let bytes = vox::encode(&grid).unwrap();
    write("my_model.vox", &bytes).unwrap();
}