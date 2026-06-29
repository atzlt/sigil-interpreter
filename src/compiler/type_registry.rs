use ahash::AHashMap;
use lasso::Spur;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeId {
    Nil,
    Bool,
    Number,
    String,
    Fn,
    Struct(u16),
}

/// Definition of a named struct type.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: Spur,
    pub fields: Vec<(Spur, TypeId)>,
    pub methods: Vec<(Spur, usize)>,
}

/// Registry of all user-declared types.
///
/// Stores struct definitions (and in the future: tags, objects, interfaces).
/// Provides name -> definition resolution for the compiler.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    pub structs: Vec<StructDef>,
    name_to_struct: AHashMap<Spur, u16>,
    // TODO: In the future, we will also have:
    // pub tags: Vec<TagDef>,
    // pub objects: Vec<ObjectDef>,
    // pub interfaces: Vec<InterfaceDef>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Structs ──

    pub fn declare_struct(&mut self, name: Spur, fields: Vec<(Spur, TypeId)>) -> TypeId {
        let def_id = self.structs.len() as u16;
        self.name_to_struct.insert(name, def_id);
        self.structs.push(StructDef {
            name,
            fields,
            methods: Vec::new(),
        });
        TypeId::Struct(def_id)
    }

    pub fn resolve_struct(&self, name: Spur) -> Option<u16> {
        self.name_to_struct.get(&name).copied()
    }

    pub fn get_struct(&self, def_id: u16) -> &StructDef {
        &self.structs[def_id as usize]
    }

    pub fn lookup_field(&self, def_id: u16, field_name: Spur) -> Option<u8> {
        let def = self.get_struct(def_id);
        def.fields
            .iter()
            .position(|(n, _)| *n == field_name)
            .map(|i| i as u8)
    }

    /// Resolve a builtin type name
    pub fn resolve_builtin_type_name(&self, name: &str) -> Option<TypeId> {
        match name {
            "Nil" => Some(TypeId::Nil),
            "Bool" => Some(TypeId::Bool),
            "Number" => Some(TypeId::Number),
            "String" => Some(TypeId::String),
            "Fn" => Some(TypeId::Fn),
            _ => None,
        }
    }

    /// Look up a method on a type. Returns `fn_id` if found.
    /// Stub — will be used when method syntax is implemented.
    #[allow(dead_code)]
    pub fn lookup_method(&self, _type_id: TypeId, _method_name: Spur) -> Option<usize> {
        None
    }

    // ── Future extension points ──

    // pub fn declare_tag(&mut self, name: Spur, payload_type: TypeId) -> TagId { ... }
    // pub fn declare_object(&mut self, name: Spur, tags: Vec<TagId>) -> ObjectId { ... }
    // pub fn declare_interface(&mut self, name: Spur, methods: Vec<...>) -> InterfaceId { ... }
}
