#[derive(Clone, Copy, Default, Debug)]
pub struct Tile {
    pub lo: u8,
    pub hi: u8,
    pub attr: u8,
    pub id: u8,
}
