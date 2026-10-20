use crate::value::{CellCoord, CellRange};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSpill {
    pub anchor: CellCoord,
    pub range: CellRange,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpillManager {
    #[serde(with = "spill_map")]
    pub active_spills: HashMap<CellCoord, DynamicSpill>,
}

impl SpillManager {
    pub fn new() -> Self {
        Self {
            active_spills: HashMap::new(),
        }
    }

    pub fn clear_anchor(&mut self, anchor: &CellCoord) {
        self.active_spills.remove(anchor);
    }

    /// Check if target spill region collides with any occupied cells or existing spills
    pub fn can_spill<F>(
        &self,
        anchor: CellCoord,
        rows: usize,
        cols: usize,
        is_cell_occupied: F,
    ) -> bool
    where
        F: Fn(CellCoord) -> bool,
    {
        if rows == 0 || cols == 0 {
            return true;
        }

        let target_range = CellRange::new(
            anchor.row,
            anchor.col,
            anchor.row + rows as u32 - 1,
            anchor.col + cols as u32 - 1,
        );

        // Check if any cell in target range (excluding anchor itself) is occupied
        for r in target_range.start_row..=target_range.end_row {
            for c in target_range.start_col..=target_range.end_col {
                let coord = CellCoord::new(r, c);
                if coord != anchor && is_cell_occupied(coord) {
                    return false;
                }
            }
        }

        // Check intersection with other existing spills
        for (other_anchor, other_spill) in &self.active_spills {
            if *other_anchor != anchor && target_range.intersects(&other_spill.range) {
                return false;
            }
        }

        true
    }

    pub fn register_spill(&mut self, anchor: CellCoord, rows: usize, cols: usize) -> DynamicSpill {
        let range = CellRange::new(
            anchor.row,
            anchor.col,
            anchor.row + rows as u32 - 1,
            anchor.col + cols as u32 - 1,
        );
        let spill = DynamicSpill {
            anchor,
            range,
            rows,
            cols,
        };
        self.active_spills.insert(anchor, spill.clone());
        spill
    }

    pub fn get_spill_at(&self, coord: &CellCoord) -> Option<&DynamicSpill> {
        self.active_spills
            .values()
            .find(|s| s.range.contains(*coord))
    }
}

mod spill_map {
    use super::*;
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        map: &HashMap<CellCoord, DynamicSpill>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let entries: std::collections::BTreeMap<String, &DynamicSpill> = map
            .iter()
            .map(|(coord, spill)| (coord.to_a1(), spill))
            .collect();
        entries.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<HashMap<CellCoord, DynamicSpill>, D::Error> {
        let entries = HashMap::<String, DynamicSpill>::deserialize(d)?;
        entries
            .into_iter()
            .map(|(key, spill)| {
                let coord = CellCoord::from_a1(&key).map_err(serde::de::Error::custom)?;
                Ok((coord, spill))
            })
            .collect()
    }
}
