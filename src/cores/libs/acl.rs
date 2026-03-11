use crate::cores::base::user::UserBase;
use crate::cores::base::user_plan::UserPlan;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub trait FallbackRule: Debug + Send + Sync {
    fn evaluate(&self, context: &AclContext) -> bool;
}

#[derive(Clone)]
pub struct AclContext {
    pub user: Arc<dyn UserBase>,
    pub resource_owner: Arc<dyn UserBase>,
    pub user_plan: Arc<dyn UserPlan>,
}

#[derive(Clone)]
pub enum AclRule {
    AllowAll,
    DenyAll,
    OwnerOnly,
    Fallback(Arc<dyn FallbackRule>),
}

impl Debug for AclRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllowAll => write!(f, "AllowAll"),
            Self::DenyAll => write!(f, "DenyAll"),
            Self::OwnerOnly => write!(f, "OwnerOnly"),
            Self::Fallback(_) => write!(f, "Fallback(DynamicLogic)"),
        }
    }
}

impl AclRule {
    pub fn evaluate(&self, context: &AclContext) -> bool {
        match self {
            Self::AllowAll => true,
            Self::DenyAll => false,
            Self::OwnerOnly => context.user.id() == context.resource_owner.id(),
            Self::Fallback(rule) => rule.evaluate(context),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AclAccess {
    pub id: String,
    pub name: String,
    pub rules: HashMap<String, AclRule>,
}

impl AclAccess {
    pub fn new<Id: AsRef<str>, Name: AsRef<str>>(id: Id, name: Name) -> Self {
        Self {
            id: id.as_ref().to_string(),
            name: name.as_ref().to_string(),
            rules: HashMap::new(),
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn remove_role<RoleId: AsRef<str>>(&mut self, role_id: RoleId) -> Option<AclRule> {
        self.rules.remove(role_id.as_ref())
    }
    pub fn add_rule<RoleId: AsRef<str>>(&mut self, role_id: RoleId, rule: AclRule) {
        self.rules.insert(role_id.as_ref().to_string(), rule);
    }
    pub fn is_permitted<RoleId: AsRef<str>>(&self, role_id: RoleId, context: &AclContext) -> bool {
        self.rules
            .get(role_id.as_ref())
            .map(|rule| rule.evaluate(context))
            .unwrap_or(false)
    }
    pub fn get_rules(&self) -> &HashMap<String, AclRule> {
        &self.rules
    }
}

impl IntoIterator for AclAccess {
    type Item = (String, AclRule);
    type IntoIter = std::collections::hash_map::IntoIter<String, AclRule>;

    fn into_iter(self) -> Self::IntoIter {
        self.rules.into_iter()
    }
}

impl Display for AclAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Acl {
    pub collections: HashMap<String, AclAccess>,
}

impl Acl {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn has_access<I: ToString>(&self, access_id: I) -> bool {
        self.collections.contains_key(&access_id.to_string())
    }
    pub fn get_access<I: ToString>(&mut self, access_id: I) -> Option<&AclAccess> {
        self.collections.get(&access_id.to_string())
    }
    pub fn get_access_mut<I: ToString>(&mut self, access_id: I) -> Option<&mut AclAccess> {
        self.collections.get_mut(&access_id.to_string())
    }
    pub fn define_access<Access: Into<AclAccess>>(&mut self, access: Access) -> &mut AclAccess {
        let access = access.into();
        let id = access.id().to_string();
        self.collections.entry(id).or_insert(access)
    }
    pub fn check<AccessId: ToString, RoleId: AsRef<str>>(
        &self,
        access_id: AccessId,
        role_id: RoleId,
        context: &AclContext,
    ) -> bool {
        self.collections
            .get(&access_id.to_string())
            .map(|set| set.is_permitted(role_id, context))
            .unwrap_or(false)
    }
}
