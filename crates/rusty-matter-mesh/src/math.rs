use rusty_matter_model::Vec3;

pub(crate) fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    if !vector.is_finite() {
        return fallback;
    }
    let length = vector.length();
    if length <= 1.0e-6 {
        fallback
    } else {
        vector / length
    }
}
