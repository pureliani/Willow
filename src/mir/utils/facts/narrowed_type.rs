use std::{any::Any, collections::BTreeSet};

use crate::{compile::interner::TypeId, mir::utils::facts::PlaceFact};

#[derive(Debug, Clone, PartialEq)]
pub struct NarrowedTypeFact {
    pub variants: BTreeSet<TypeId>,
}

impl PlaceFact for NarrowedTypeFact {
    fn clone_fact(&self) -> Box<dyn PlaceFact> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_fact(&self, other: &dyn PlaceFact) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn merge(&self, other: &dyn PlaceFact) -> Option<Box<dyn PlaceFact>> {
        let other = other.as_any().downcast_ref::<Self>()?;
        let merged: BTreeSet<_> = self.variants.union(&other.variants).cloned().collect();
        Some(Box::new(NarrowedTypeFact { variants: merged }))
    }

    fn intersect(&self, other: &dyn PlaceFact) -> Option<Box<dyn PlaceFact>> {
        let other = other.as_any().downcast_ref::<Self>()?;
        let intersected: BTreeSet<_> = self
            .variants
            .intersection(&other.variants)
            .cloned()
            .collect();
        if intersected.is_empty() {
            None
        } else {
            Some(Box::new(NarrowedTypeFact {
                variants: intersected,
            }))
        }
    }
}
