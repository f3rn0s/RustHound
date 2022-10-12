use bitflags::bitflags;
use nom::{
    number::complete::{le_u128, le_u16, le_u32, le_u8},
    *,
};

use crate::enums::constants::*;

// https://github.com/fox-it/dissect.cstruct/blob/master/examples/secdesc.py
// http://www.selfadsi.org/deep-inside/ad-security-descriptors.htm#SecurityDescriptorStructure
// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/20233ed8-a6c6-4097-aafa-dd545ed24428?redirectedfrom=MSDN

/// Structure for Security Descriptor network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/7d4dac05-9cef-4563-a058-f108abecce1d>
#[derive(Debug)]
pub struct SecurityDescriptor {
    pub revision: u8,
    pub sbz1: u8,
    pub control: u16,
    pub offset_owner: u32,
    pub offset_group: u32,
    pub offset_sacl: u32,
    pub offset_dacl: u32,
}

impl SecurityDescriptor {
    named!(
        pub parse<Self>,
        do_parse!(
            revision: le_u8
            >> sbz1: le_u8
            >> control: le_u16
            >> offset_owner: le_u32
            >> offset_group: le_u32
            >> offset_sacl: le_u32
            >> offset_dacl: le_u32
            >> ({
                SecurityDescriptor {
                    revision,
                    sbz1,
                    control,
                    offset_owner,
                    offset_group,
                    offset_sacl,
                    offset_dacl,
                }
            })
        )
    );
}

/// Strcuture for Sid Identified Authority network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/c6ce4275-3d90-4890-ab3a-514745e4637e>
#[derive(Debug, Clone)]
pub struct LdapSidIdentifiedAuthority {
    pub value: Vec<u8>,
}

impl LdapSidIdentifiedAuthority {
    named!(
        pub parse<Self>,
        do_parse!(
            value: take!(6)
            >> ({
                LdapSidIdentifiedAuthority {
                    value:value.to_vec()
                }
            })
        )
    );
}

/// Structure for LDAPSID network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/f992ad60-0fe4-4b87-9fed-beb478836861>
#[derive(Clone, Debug)]
pub struct LdapSid {
    pub revision: u8,
    pub sub_authority_count: u8,
    pub identifier_authority: LdapSidIdentifiedAuthority,
    pub sub_authority: Vec<u32>,
}

impl LdapSid {
    named!(
        pub parse<Self>,
        do_parse!(
            revision: le_u8
            >> sub_authority_count: le_u8
            >> identifier_authority: call!(LdapSidIdentifiedAuthority::parse)
            >> sub_authority: count!(le_u32 ,sub_authority_count as usize)
            >> ({
                LdapSid {
                    revision,
                    sub_authority_count,
                    identifier_authority,
                    sub_authority
                }
            })
        )
    );
}

/// Structure for Acl network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/20233ed8-a6c6-4097-aafa-dd545ed24428>
#[derive(Debug)]
pub struct Acl {
    pub acl_revision: u8,
    pub sbz1: u8,
    pub acl_size: u16,
    pub ace_count: u16,
    pub sbz2: u16,
    // Length = acl_size
    pub data: Vec<Ace>,
}

impl Acl {
    named!(
        pub parse<Self>,
        do_parse!(
            acl_revision: le_u8
            >> sbz1: le_u8
            >> acl_size: le_u16
            >> ace_count: le_u16
            >> sbz2: le_u16
            >> data: count!(Ace::parse, ace_count as usize)
            >> ({
                Acl {
                    acl_revision,
                    sbz1,
                    acl_size,
                    ace_count,
                    sbz2,
                    data
                }
            })
        )
    );
}

/// Structure for Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/628ebb1d-c509-4ea0-a10f-77ef97ca4586>
#[derive(Debug)]
pub struct Ace {
    pub ace_type: u8,
    pub ace_flags: u8,
    pub ace_size: u16,
    // Lenght = ace_size-4
    pub data: AceFormat,
}

impl Ace {
    named!(
        pub parse<Self>,
        do_parse!(
            ace_type: le_u8
            >> ace_flags: le_u8
            >> ace_size: le_u16
            >> data: switch!(value!(ace_type as u8),
                ACCESS_ALLOWED_ACE_TYPE => call!(AccessAllowedAce::parse)|
                ACCESS_DENIED_ACE_TYPE => call!(AccessAllowedAce::parse)|
                ACCESS_ALLOWED_OBJECT_ACE_TYPE => call!(AccessAllowedObjectAce::parse)|
                ACCESS_DENIED_OBJECT_ACE_TYPE => call!(AccessAllowedObjectAce::parse)
            )
            >> ({
                Ace {
                    ace_type,
                    ace_flags,
                    ace_size,
                    data
                }
            })
        )
    );
}

/// Enum to get the same ouput for data switch in Ace structure.
#[derive(Clone, Debug)]
pub enum AceFormat {
    AceAllowed(AccessAllowedAce),
    AceObjectAllowed(AccessAllowedObjectAce),
    Empty,
}

impl AceFormat {
    pub fn get_mask(value: AceFormat) -> Option<u32> {
        match value {
            AceFormat::AceAllowed(ace) => Some(ace.mask),
            AceFormat::AceObjectAllowed(ace) => Some(ace.mask),
            AceFormat::Empty => None,
        }
    }

    pub fn get_sid(value: AceFormat) -> Option<LdapSid> {
        match value {
            AceFormat::AceAllowed(ace) => Some(ace.sid),
            AceFormat::AceObjectAllowed(ace) => Some(ace.sid),
            AceFormat::Empty => None,
        }
    }

    pub fn get_flags(value: AceFormat) -> Option<ObjectAceFlags> {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => Some(ace.flags),
            AceFormat::Empty => None,
        }
    }

    pub fn get_object_type(value: AceFormat) -> Option<u128> {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => ace.object_type,
            AceFormat::Empty => None,
        }
    }

    pub fn get_inherited_object_type(value: AceFormat) -> Option<u128> {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => ace.inherited_object_type,
            AceFormat::Empty => None,
        }
    }
}

/// Structure for Access Allowed Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/72e7c7ea-bc02-4c74-a619-818a16bf6adb>
#[derive(Clone, Debug)]
pub struct AccessAllowedAce {
    pub mask: u32,
    pub sid: LdapSid,
}

impl AccessAllowedAce {
    named!(
        pub parse<AceFormat>,
        do_parse!(
            mask: le_u32
            >> sid: call!(LdapSid::parse)
            >> ({
                AceFormat::AceAllowed (
                    AccessAllowedAce {
                    mask,
                    sid,
                }
                )
            })
        )
    );
}

/// Structure for Access Allowed Object Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/c79a383c-2b3f-4655-abe7-dcbb7ce0cfbe>
#[derive(Clone, Debug)]
pub struct AccessAllowedObjectAce {
    pub mask: u32,
    pub flags: ObjectAceFlags,
    pub object_type: Option<u128>,
    pub inherited_object_type: Option<u128>,
    //char    object_type[flags & 1 * 16];
    //char    Inheritedobject_type[flags & 2 * 8];
    pub sid: LdapSid,
}

impl AccessAllowedObjectAce {
    named!(
        pub parse<AceFormat>,
        do_parse!(
            mask: le_u32
            >> flags: call!(ObjectAceFlags::parse)
            >> object_type:
                cond!(flags.contains(ObjectAceFlags::ACE_OBJECT_PRESENT),le_u128)
            >> inherited_object_type:
                cond!(flags.contains(ObjectAceFlags::ACE_INHERITED_OBJECT_PRESENT),le_u128)
            >> sid: call!(LdapSid::parse)
            >> ({
                AceFormat::AceObjectAllowed (
                    AccessAllowedObjectAce {
                    mask,
                    flags,
                    object_type,
                    inherited_object_type,
                    sid,
                }
            )
            })
        )
    );
}

bitflags! {
    /// AceFlags
    pub struct ObjectAceFlags : u32 {
        const ACE_OBJECT_PRESENT= 0x0001;
        const ACE_INHERITED_OBJECT_PRESENT= 0x0002;
    }
}

impl ObjectAceFlags {
    named!(
        pub parse<ObjectAceFlags>,
        do_parse!(
            flags: le_u32
            >> ({
                // Will never fail
                ObjectAceFlags::from_bits(flags).unwrap()
            })
        )
    );
}




/// Test functions
#[test]
#[rustfmt::skip]
pub fn test_secdesc() {

    let original = vec![
        // SECURITY_DECRIPTOR [0..15]
            // revision
            1,
            // Internal
            0,
            // control flags
            4, 140,
            // offset_owner
            120, 9, 0, 0,
            // offset_group
            0, 0, 0, 0,
            // offset_sacl
            0, 0, 0, 0,
            // offset_dacl
            20, 0, 0, 0
    ];

    let result          = SecurityDescriptor::parse(&original).unwrap().1;
    assert_eq!(result.revision, 1);
}

#[test]
#[rustfmt::skip]
pub fn test_ace() {

    let original_ace = vec![
        // Type
        0x00,
        // Flag
        0x12,
        // Size
        0x18, 0x00,
        // Data
            // Mask
            0xbd, 0x01, 0x0f, 0x00,
            // Sid
            0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x20, 0x02, 0x00, 0x00
    ];

    let result          = Ace::parse(&original_ace).unwrap().1;
    assert_eq!(result.ace_type, 0);
    println!("ACE_ALLOWED: {:?}",result);


    let original_ace_object = vec![
        // Type
        0x05,
        // Flag
        0x12,
        // Size
        0x2c, 0x00,
        // Data
            // Mask
            0x94, 0x00, 0x02, 0x00,
            // Ace Object
                // Flags
                0x02, 0x00, 0x00, 0x00,
                // Inherited GUID
                0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2,
            // Sid
            0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00
    ];

    let result          = Ace::parse(&original_ace_object).unwrap().1;
    assert_eq!(result.ace_type, 5);
    println!("ACE_ALLOWED_OBJECT: {:?}",result);
}

#[test]
#[rustfmt::skip]
pub fn test_acl_admin() {

    //Adminstrateur test Acl
    let original_acl = vec![ 0x04, 0x00, 0x74, 0x04, 0x18, 0x00, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x7f, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x05, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1d, 0xb1, 0xa9, 0x46, 0xae, 0x60, 0x5a, 0x40, 0xb7, 0xe8, 0xff, 0x8a, 0x58, 0xd4, 0x56, 0xd2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x30, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1c, 0x9a, 0xb6, 0x6d, 0x22, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x62, 0xbc, 0x05, 0x58, 0xc9, 0xbd, 0x28, 0x44, 0xa5, 0xe2, 0x85, 0x6a, 0x0f, 0x4c, 0x18, 0x5e, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x02, 0x28, 0x00, 0x30, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xde, 0x47, 0xe6, 0x91, 0x6f, 0xd9, 0x70, 0x4b, 0x95, 0x57, 0xd6, 0x3f, 0xf4, 0xf3, 0xcc, 0xd8, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0xbf, 0x01, 0x0e, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0xbf, 0x01, 0x0e, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x07, 0x02, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0xbf, 0x01, 0x0f, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x20, 0x02, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x94, 0x00, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x00, 0x00, 0x00 ];

    let result          = Acl::parse(&original_acl).unwrap().1;
    assert_eq!(result.acl_size, 1140);
    println!("ACL: {:?}",result);
}
