pub mod narrowed_type;

use std::any::{Any, TypeId as RustTypeId};
use std::collections::HashMap;
use std::fmt::Debug;

pub trait PlaceFact: Debug + 'static {
    fn clone_fact(&self) -> Box<dyn PlaceFact>;
    fn as_any(&self) -> &dyn Any;
    fn eq_fact(&self, other: &dyn PlaceFact) -> bool;

    /// Control Flow Join (Merge)
    fn merge(&self, other: &dyn PlaceFact) -> Option<Box<dyn PlaceFact>>;

    /// Control Flow Meet (Intersect)
    fn intersect(&self, other: &dyn PlaceFact) -> Option<Box<dyn PlaceFact>>;
}

impl Clone for Box<dyn PlaceFact> {
    fn clone(&self) -> Self {
        self.clone_fact()
    }
}

impl PartialEq for Box<dyn PlaceFact> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_fact(other.as_ref())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FactSet {
    pub facts: HashMap<RustTypeId, Box<dyn PlaceFact>>,
}

impl FactSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: PlaceFact>(&mut self, fact: T) {
        self.facts.insert(RustTypeId::of::<T>(), Box::new(fact));
    }

    pub fn get<T: PlaceFact>(&self) -> Option<&T> {
        self.facts
            .get(&RustTypeId::of::<T>())
            .and_then(|f| f.as_any().downcast_ref::<T>())
    }

    pub fn remove<T: PlaceFact>(&mut self) {
        self.facts.remove(&RustTypeId::of::<T>());
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut result = FactSet::new();
        for (type_id, self_fact) in &self.facts {
            if let Some(other_fact) = other.facts.get(type_id) {
                if let Some(merged) = self_fact.merge(other_fact.as_ref()) {
                    result.facts.insert(*type_id, merged);
                }
            }
        }
        result
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (type_id, other_fact) in &other.facts {
            if let Some(self_fact) = result.facts.get(type_id) {
                if let Some(intersected) = self_fact.intersect(other_fact.as_ref()) {
                    result.facts.insert(*type_id, intersected);
                } else {
                    result.facts.remove(type_id);
                }
            } else {
                result.facts.insert(*type_id, other_fact.clone());
            }
        }
        result
    }
}
