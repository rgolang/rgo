pub const ENV_METADATA_FIELD_SIZE: usize = 8;
pub const ENV_METADATA_POINTER_COUNT_OFFSET: usize = ENV_METADATA_FIELD_SIZE * 2;
pub const ENV_METADATA_POINTER_LIST_OFFSET: usize = ENV_METADATA_FIELD_SIZE * 3;
pub const ENV_METADATA_SIZE: usize = ENV_METADATA_POINTER_LIST_OFFSET;

pub fn env_metadata_size(pointer_count: usize) -> usize {
    ENV_METADATA_SIZE + pointer_count * ENV_METADATA_FIELD_SIZE
}
