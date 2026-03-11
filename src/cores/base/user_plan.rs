use std::fmt::Debug;

pub trait UserPlan: Debug + Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn has_feature(&self, feature: dyn AsRef<str>) -> bool;
}
