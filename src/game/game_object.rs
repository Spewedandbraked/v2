use glam::Mat4;

pub struct GameObject {
    pub model_index: usize,
    pub transform: Mat4,
    pub material_index: usize,
    pub is_animated: bool,
    pub bone_matrices: Option<Vec<Mat4>>,
    pub is_visible: bool,
}
