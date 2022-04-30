use crate::core::ppu::Color;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Line([Color; 160]);

impl Default for Line {
    fn default() -> Self {
        Self([Color::White; 160])
    }
}

impl Index<usize> for Line {
    type Output = Color;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Line {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[derive(Debug, Clone)]
pub struct Buffer([Line; 144]);

impl Buffer {
    pub(super) fn boxed() -> Box<Self> {
        Box::new(Self::default())
    }

    pub fn height() -> usize {
        144
    }

    pub fn width() -> usize {
        160
    }

    pub fn flatten(&self) -> impl Iterator<Item = Color> + '_ {
        self.0.iter().flat_map(|b| b.0)
    }
}

impl Index<usize> for Buffer {
    type Output = Line;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Buffer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self([(); 144].map(|_| Line::default()))
    }
}
