use glam::*;

pub struct Octant {
    // Data layout:
    // [  0-7] bool is_leaf
    // [ 8-15] u8 r
    // [16-23] u8 g
    // [24-31] u8 b
    pub data: u32,

    // TODO: An option stores 4 extra bytes in a few cases, needs to be tested.
    // We use Option<&Octant> here instead of a pointer, because it's a lot safer.
    pub children: [Option<Box<Octant>>; 8],

    // Node position, size and depth
    // TODO: this can be calculated implicitly, might be worth the memory saving?
    pub center: Vec3,
    pub half_size: Vec3,
    pub depth: u8,
}

impl Octant {
    pub fn leaf(center: Vec3, half_size: Vec3, depth: u8, r: u8, g: u8, b: u8) -> Self {
        let mut data: u32 = 0;
        data |= true as u32;
        data |= (r as u32) << 8;
        data |= (g as u32) << 16;
        data |= (b as u32) << 24;
        Self {
            data: data,
            children: [None, None, None, None, None, None, None, None],
            center: center,
            half_size: half_size,
            depth: depth,
        }
    }

    pub fn empty(center: Vec3, half_size: Vec3, depth: u8) -> Self {
        Self {
            data: 0,
            children: [None, None, None, None, None, None, None, None],
            center: center,
            half_size: half_size,
            depth: depth,
        }
    }

    pub fn set_leaf(&mut self, leaf: bool) {
        self.data = self.data & (!0xFFu32);
        self.data |= leaf as u32;
    }

    pub fn is_leaf(&self) -> bool {
        (self.data & 0xFF) != 0
    }

    pub fn color(&self) -> (u8, u8, u8) {
        let r = (self.data & (0xFF << 8)) >> 8;
        let g = (self.data & (0xFF << 16)) >> 16;
        let b = (self.data & (0xFF << 24)) >> 24;
        (r as u8, g as u8, b as u8)
    }
}

#[derive(PartialEq)]
pub enum OctantFillState {
    Empty,
    ContainsVoxel,
    Full,
}

pub struct VoxelOctree {
    pub root: Octant,
}

impl VoxelOctree {
    pub fn empty(center: Vec3, size: Vec3) -> Self {
        let root = Octant::empty(center, size / 2.0, 0);
        Self {
            root: root,
        }
    }

    fn gen_octant<F>(octant: &mut Octant, max_depth: u8, nodes_generated: &mut usize, contains_voxel: F)
    where
        F: Fn(Vec3, Vec3, Vec3) -> OctantFillState + Copy
    {
        let mut i = 0;
        for ix in 0..2 {
            for iy in 0..2 {
                for iz in 0..2 {
                    let x = (ix * 2 - 1) as f32;
                    let y = (iy * 2 - 1) as f32;
                    let z = (iz * 2 - 1) as f32;
                    let sign = vec3(x,y,z);
                    let child_pos = octant.center + octant.half_size * sign * 0.5;
                    let child_half_size = octant.half_size / 2.0;
                    let child_inner = child_pos + child_half_size * -sign;
                    let child_outer = child_pos + child_half_size * sign;
                    let vox_status = contains_voxel(child_pos, child_inner, child_outer);
                    match vox_status {
                        OctantFillState::Empty => {},
                        OctantFillState::ContainsVoxel => {
                            if octant.depth + 1 < max_depth {
                                //We have not yet reached max depth
                                octant.children[i] = Some(Box::new(Octant::empty(child_pos, child_half_size, octant.depth + 1)));
                                VoxelOctree::gen_octant(octant.children[i].as_mut().unwrap(), max_depth, nodes_generated, contains_voxel);
                            } else {
                                //We went to the max depth, so just mark the last nodes as leaf if they are inside the sphere
                                octant.children[i] = Some(Box::new(Octant::leaf(child_pos, child_half_size, octant.depth + 1, 255,0,255)));
                            }
                            i += 1;
                            *nodes_generated += 1;
                        },
                        OctantFillState::Full => {
                            //The whole octant is filled. We can just make it a leaf node immediately
                            octant.children[i] = Some(Box::new(Octant::leaf(child_pos, child_half_size, octant.depth + 1, 255,0,255)));
                            i += 1;
                            *nodes_generated += 1;
                        },
                    }
                }
            }
        }
    }

    pub fn generate<F>(&mut self, max_depth: u8, contains_voxel: F) -> usize
    where
        F: Fn(Vec3, Vec3, Vec3) -> OctantFillState + Copy
    {
        let mut nodes_generated = 0;
        VoxelOctree::gen_octant(&mut self.root, max_depth, &mut nodes_generated, contains_voxel);
        nodes_generated
    }

    pub fn generate_sphere(&mut self, radius: f32, max_depth: u8) {
        let mut nodes_generated = 0;

        VoxelOctree::gen_octant(&mut self.root, max_depth, &mut nodes_generated, |center, inner, outer| {
            let mut status = OctantFillState::Empty;
            let min = inner.abs().min(outer.abs());
            let max = inner.abs().max(outer.abs());
            if min.length() < radius {
                status = OctantFillState::ContainsVoxel;
                if max.length() < radius {
                    status = OctantFillState::Full;
                }
            }
            status
        });

        trace!("Nodes generated: {}", nodes_generated);
    }

    fn add_octant_cube(octant: &Octant, indices: &mut Vec<u32>, positions: &mut Vec<f64>) {
        let v1 = octant.center + octant.half_size * vec3(-1.0,-1.0,-1.0);
        let v2 = octant.center + octant.half_size * vec3( 1.0,-1.0,-1.0);
        let v3 = octant.center + octant.half_size * vec3( 1.0, 1.0,-1.0);
        let v4 = octant.center + octant.half_size * vec3(-1.0, 1.0,-1.0);

        let v5 = octant.center + octant.half_size * vec3(-1.0,-1.0, 1.0);
        let v6 = octant.center + octant.half_size * vec3( 1.0,-1.0, 1.0);
        let v7 = octant.center + octant.half_size * vec3( 1.0, 1.0, 1.0);
        let v8 = octant.center + octant.half_size * vec3(-1.0, 1.0, 1.0);

        let cube_idx_start = positions.len() / 3;
        let v1_idx = cube_idx_start;
        let v2_idx = cube_idx_start + 1;
        let v3_idx = cube_idx_start + 2;
        let v4_idx = cube_idx_start + 3;
        let v5_idx = cube_idx_start + 4;
        let v6_idx = cube_idx_start + 5;
        let v7_idx = cube_idx_start + 6;
        let v8_idx = cube_idx_start + 7;

        positions.push(v1.x as f64);
        positions.push(v1.y as f64);
        positions.push(v1.z as f64);

        positions.push(v2.x as f64);
        positions.push(v2.y as f64);
        positions.push(v2.z as f64);

        positions.push(v3.x as f64);
        positions.push(v3.y as f64);
        positions.push(v3.z as f64);

        positions.push(v4.x as f64);
        positions.push(v4.y as f64);
        positions.push(v4.z as f64);

        positions.push(v5.x as f64);
        positions.push(v5.y as f64);
        positions.push(v5.z as f64);

        positions.push(v6.x as f64);
        positions.push(v6.y as f64);
        positions.push(v6.z as f64);

        positions.push(v7.x as f64);
        positions.push(v7.y as f64);
        positions.push(v7.z as f64);

        positions.push(v8.x as f64);
        positions.push(v8.y as f64);
        positions.push(v8.z as f64);

        //Back
        indices.push(v1_idx as u32);
        indices.push(v2_idx as u32);
        indices.push(v3_idx as u32);
        //
        indices.push(v1_idx as u32);
        indices.push(v3_idx as u32);
        indices.push(v4_idx as u32);

        //Front
        indices.push(v5_idx as u32);
        indices.push(v8_idx as u32);
        indices.push(v7_idx as u32);
        //
        indices.push(v7_idx as u32);
        indices.push(v6_idx as u32);
        indices.push(v5_idx as u32);

        //Right
        indices.push(v2_idx as u32);
        indices.push(v6_idx as u32);
        indices.push(v7_idx as u32);
        //
        indices.push(v7_idx as u32);
        indices.push(v3_idx as u32);
        indices.push(v2_idx as u32);

        //Left
        indices.push(v4_idx as u32);
        indices.push(v8_idx as u32);
        indices.push(v5_idx as u32);
        //
        indices.push(v5_idx as u32);
        indices.push(v1_idx as u32);
        indices.push(v4_idx as u32);

        //Top
        indices.push(v4_idx as u32);
        indices.push(v3_idx as u32);
        indices.push(v7_idx as u32);
        //
        indices.push(v7_idx as u32);
        indices.push(v8_idx as u32);
        indices.push(v4_idx as u32);

        //Bottom
        indices.push(v5_idx as u32);
        indices.push(v6_idx as u32);
        indices.push(v2_idx as u32);
        //
        indices.push(v2_idx as u32);
        indices.push(v1_idx as u32);
        indices.push(v5_idx as u32);
    }

    fn export_octant_walk(octant: &Octant, indices: &mut Vec<u32>, positions: &mut Vec<f64>) {
        if octant.is_leaf() {
            VoxelOctree::add_octant_cube(octant, indices, positions);
        }

        for i in 0..8 {
            match &octant.children[i] {
                Some(child) => VoxelOctree::export_octant_walk(&child, indices, positions),
                None => {},
            }
        }
    }

    pub fn export_mesh(&self) {
        use tri_mesh::prelude::*;

        let mut indices: Vec<u32> = Vec::new();
        let mut positions: Vec<f64> = Vec::new();

        VoxelOctree::export_octant_walk(&self.root, &mut indices, &mut positions);

        trace!("Indices: {}", indices.len());
        trace!("Positions: {}", positions.len());

        let mesh = MeshBuilder::new().with_indices(indices).with_positions(positions).build().unwrap();
        std::fs::write("octree.obj", mesh.parse_as_obj()).unwrap();
    }
}
