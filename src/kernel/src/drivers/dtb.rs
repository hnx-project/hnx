//! 设备树(blob)解析模块
//!
//! 提供对设备树二进制格式(DTB)的完整解析能力，包括：
//! - 设备树节点遍历
//! - 属性解析
//! - 设备匹配
//! - 设备查找

use core::str;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;

/// 设备树解析器
pub struct DtbParser {
    /// DTB数据指针
    dtb_ptr: usize,
    /// 结构块偏移
    off_struct: usize,
    /// 字符串块偏移
    off_strings: usize,
    /// 结构块大小
    size_struct: usize,
    /// 字符串块数据
    strings: &'static [u8],
    /// DTB数据块
    blob: &'static [u8],
}

/// 设备树节点
#[derive(Debug, Clone)]
pub struct DtbNode {
    /// 节点名称
    pub name: &'static str,
    /// 节点路径
    pub path: &'static str,
    /// 节点偏移
    pub offset: usize,
}

/// 设备树属性
#[derive(Debug, Clone)]
pub struct DtbProperty {
    /// 属性名称
    pub name: &'static str,
    /// 属性值
    pub value: &'static [u8],
}

/// 设备匹配条件
#[derive(Debug)]
pub struct DeviceMatch {
    /// 兼容性字符串列表
    pub compatible: Option<&'static [&'static str]>,
    /// 设备类型
    pub device_type: Option<&'static str>,
    /// 特定属性值匹配
    pub properties: Option<&'static [(&'static str, &'static [u8])]>,
}

impl DtbParser {
    /// 创建新的DTB解析器
    pub fn new(dtb_ptr: usize) -> Option<Self> {
        if dtb_ptr == 0 {
            return None;
        }

        unsafe {
            let hdr = core::slice::from_raw_parts(dtb_ptr as *const u8, 40);
            let magic = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
            
            // 检查设备树魔数
            if magic != 0xD00D_FEEE {
                return None;
            }

            let off_struct = u32::from_be_bytes([hdr[8], hdr[9], hdr[10], hdr[11]]) as usize;
            let off_strings = u32::from_be_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]) as usize;
            let size_struct = u32::from_be_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]) as usize;
            
            let blob = core::slice::from_raw_parts(dtb_ptr as *const u8, off_struct + size_struct);
            let strings = core::slice::from_raw_parts((dtb_ptr + off_strings) as *const u8, 1 << 20);

            Some(DtbParser {
                dtb_ptr,
                off_struct,
                off_strings,
                size_struct,
                strings,
                blob,
            })
        }
    }

    /// 获取大端32位值
    fn be32(&self, bytes: &[u8]) -> u32 {
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    /// 获取大端64位值
    fn be64(&self, bytes: &[u8]) -> u64 {
        u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7]
        ])
    }

    /// 获取字符串表中的字符串
    fn get_string(&self, offset: usize) -> Option<&'static str> {
        if offset >= self.strings.len() {
            return None;
        }

        let mut end = offset;
        while end < self.strings.len() && self.strings[end] != 0 {
            end += 1;
        }

        let slice = &self.strings[offset..end];
        str::from_utf8(slice).ok()
    }

    /// 解析设备树节点
    pub fn parse_node<F>(&self, mut callback: F) 
    where 
        F: FnMut(&DtbNode, &[DtbProperty])
    {
        let mut i = self.off_struct;
        let mut path_stack: ArrayVec<&str, 32> = ArrayVec::new();
        
        while i + 4 <= self.blob.len() {
            let token = self.be32(&self.blob[i..i + 4]);
            i += 4;

            match token {
                // FDT_BEGIN_NODE
                1 => {
                    // 读取节点名称
                    let name_start = i;
                    while i < self.blob.len() && self.blob[i] != 0 {
                        i += 1;
                    }
                    
                    let name_slice = &self.blob[name_start..i];
                    if let Ok(name) = str::from_utf8(name_slice) {
                        // 对齐到4字节边界
                        while i % 4 != 0 {
                            i += 1;
                        }
                        
                        // 构建路径
                        path_stack.push(name);
                        let path = if path_stack.len() == 1 && name == "" {
                            "/".to_string()
                        } else if path_stack.len() == 1 {
                            name.to_string()
                        } else {
                            let mut path_parts = Vec::new();
                            for part in path_stack.iter() {
                                if !part.is_empty() {
                                    path_parts.push(*part);
                                }
                            }
                            path_parts.join("/")
                        };
                        
                        let node = DtbNode {
                            name,
                            path: Box::leak(path.into_boxed_str()),
                            offset: name_start,
                        };
                        
                        // 解析属性
                        let properties = self.parse_properties(&mut i);
                        callback(&node, &properties);
                    } else {
                        // 对齐到4字节边界
                        while i % 4 != 0 {
                            i += 1;
                        }
                    }
                }
                
                // FDT_END_NODE
                2 => {
                    if !path_stack.is_empty() {
                        path_stack.pop();
                    }
                    // 对齐到4字节边界
                    while i % 4 != 0 {
                        i += 1;
                    }
                }
                
                // FDT_PROP
                3 => {
                    // 跳过属性，已经在parse_properties中处理
                    while i % 4 != 0 {
                        i += 1;
                    }
                }
                
                // FDT_NOP
                4 => {
                    // NOP，直接跳过
                }
                
                // FDT_END
                9 => {
                    break;
                }
                
                _ => {
                    // 未知token，跳过
                    while i % 4 != 0 {
                        i += 1;
                    }
                }
            }
        }
    }

    /// 解析节点属性
    fn parse_properties(&self, i: &mut usize) -> ArrayVec<DtbProperty, 16> {
        let mut properties = ArrayVec::new();
        
        // 保存当前位置
        let mut pos = *i;
        
        while pos + 4 <= self.blob.len() {
            let token = self.be32(&self.blob[pos..pos + 4]);
            
            // 如果遇到节点结束标记，停止解析
            if token == 2 || token == 1 || token == 9 {
                break;
            }
            
            // 只处理属性token
            if token == 3 {
                pos += 4;
                
                if pos + 8 > self.blob.len() {
                    break;
                }
                
                let len = self.be32(&self.blob[pos..pos + 4]) as usize;
                let nameoff = self.be32(&self.blob[pos + 4..pos + 8]) as usize;
                pos += 8;
                
                if pos + len > self.blob.len() {
                    break;
                }
                
                // 获取属性名称
                if let Some(name) = self.get_string(nameoff) {
                    let value = &self.blob[pos..pos + len];
                    properties.push(DtbProperty { name, value });
                }
                
                pos += len;
                
                // 对齐到4字节边界
                while pos % 4 != 0 {
                    pos += 1;
                }
            } else {
                // 对齐到4字节边界
                pos += 4;
                while pos % 4 != 0 {
                    pos += 1;
                }
            }
        }
        
        *i = pos;
        properties
    }

    /// 查找匹配的设备节点
    pub fn find_matching_devices(&self, match_criteria: &DeviceMatch) -> ArrayVec<DtbNode, 8> {
        let mut matching_nodes = ArrayVec::new();
        
        self.parse_node(|node, properties| {
            if self.node_matches(node, properties, match_criteria) {
                matching_nodes.push(node.clone());
            }
        });
        
        matching_nodes
    }

    /// 根据兼容性字符串查找设备节点
    pub fn find_devices_by_compatible(&self, compatible: &str) -> ArrayVec<DtbNode, 8> {
        let match_criteria = DeviceMatch {
            compatible: Some(&[compatible]),
            device_type: None,
            properties: None,
        };
        
        self.find_matching_devices(&match_criteria)
    }

    /// 根据设备类型查找设备节点
    pub fn find_devices_by_type(&self, device_type: &str) -> ArrayVec<DtbNode, 8> {
        let match_criteria = DeviceMatch {
            compatible: None,
            device_type: Some(device_type),
            properties: None,
        };
        
        self.find_matching_devices(&match_criteria)
    }

    /// 根据属性值查找设备节点
    pub fn find_devices_by_property(&self, property_name: &str, property_value: &[u8]) -> ArrayVec<DtbNode, 8> {
        let props = &[(property_name, property_value)];
        let match_criteria = DeviceMatch {
            compatible: None,
            device_type: None,
            properties: Some(props),
        };
        
        self.find_matching_devices(&match_criteria)
    }

    /// 获取所有子节点
    pub fn get_child_nodes(&self, parent_path: &str) -> ArrayVec<DtbNode, 16> {
        let mut child_nodes = ArrayVec::new();
        
        self.parse_node(|node, _properties| {
            // 检查节点是否是指定父节点的直接子节点
            if node.path.len() > parent_path.len() && 
               node.path.starts_with(parent_path) &&
               node.path[parent_path.len()..].starts_with("/") {
                // 确保这是直接子节点，而不是孙子节点
                let remainder = &node.path[parent_path.len() + 1..];
                if !remainder.contains('/') {
                    child_nodes.push(node.clone());
                }
            } else if parent_path == "/" && node.path.chars().filter(|&c| c == '/').count() == 1 {
                // 特殊情况：获取根节点的直接子节点
                child_nodes.push(node.clone());
            }
        });
        
        child_nodes
    }

    /// 获取所有设备节点
    pub fn get_all_device_nodes(&self) -> ArrayVec<DtbNode, 32> {
        let mut device_nodes = ArrayVec::new();
        
        self.parse_node(|node, properties| {
            // 检查节点是否有compatible属性，这通常表示它是一个设备节点
            if properties.iter().any(|prop| prop.name == "compatible") {
                device_nodes.push(node.clone());
            }
        });
        
        device_nodes
    }

    /// 检查节点是否匹配给定条件
    fn node_matches(&self, _node: &DtbNode, properties: &[DtbProperty], match_criteria: &DeviceMatch) -> bool {
        // 检查兼容性字符串
        if let Some(compat_list) = match_criteria.compatible {
            if let Some(compat_prop) = properties.iter().find(|p| p.name == "compatible") {
                let compat_str = match str::from_utf8(compat_prop.value) {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                
                // 检查是否有任何兼容性字符串匹配
                let mut found = false;
                for compat in compat_list {
                    if compat_str.contains(compat) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // 检查设备类型
        if let Some(device_type) = match_criteria.device_type {
            if let Some(type_prop) = properties.iter().find(|p| p.name == "device_type") {
                if let Ok(type_str) = str::from_utf8(type_prop.value) {
                    if type_str != device_type {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // 检查特定属性值
        if let Some(props) = match_criteria.properties {
            for (prop_name, prop_value) in props {
                if let Some(prop) = properties.iter().find(|p| p.name == *prop_name) {
                    if prop.value != *prop_value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        
        true
    }

    /// 获取指定路径的节点
    pub fn get_node_by_path(&self, path: &str) -> Option<(DtbNode, ArrayVec<DtbProperty, 16>)> {
        let mut result = None;
        
        self.parse_node(|node, properties| {
            if node.path == path {
                result = Some((node.clone(), properties.clone()));
            }
        });
        
        result
    }

    /// 获取指定节点的属性值
    pub fn get_property_value(&self, node_path: &str, property_name: &str) -> Option<&'static [u8]> {
        self.get_node_by_path(node_path)
            .and_then(|(_, properties)| {
                properties.iter()
                    .find(|prop| prop.name == property_name)
                    .map(|prop| prop.value)
            })
    }

    /// 解析reg属性获取内存区域
    pub fn parse_reg_property(&self, reg_data: &[u8]) -> ArrayVec<(u64, u64), 8> {
        let mut regions = ArrayVec::new();
        let mut i = 0;
        
        // 检查数据长度是否足够
        if reg_data.len() < 16 {
            return regions;
        }
        
        while i + 16 <= reg_data.len() {
            let address = self.be64(&reg_data[i..i + 8]);
            let size = self.be64(&reg_data[i + 8..i + 16]);
            regions.push((address, size));
            i += 16;
        }
        
        regions
    }

    /// 解析interrupts属性获取中断信息
    pub fn parse_interrupts_property(&self, interrupts_data: &[u8]) -> ArrayVec<u32, 8> {
        let mut interrupts = ArrayVec::new();
        let mut i = 0;
        
        // 检查数据长度是否足够
        if interrupts_data.len() < 4 {
            return interrupts;
        }
        
        while i + 4 <= interrupts_data.len() {
            let irq = self.be32(&interrupts_data[i..i + 4]);
            interrupts.push(irq);
            i += 4;
        }
        
        interrupts
    }

    /// 解析compatible属性获取兼容性字符串列表
    pub fn parse_compatible_property(&self, compat_data: &[u8]) -> ArrayVec<&'static str, 8> {
        let mut compatibles = ArrayVec::new();
        let mut i = 0;
        
        while i < compat_data.len() {
            // 查找字符串结束符
            let mut end = i;
            while end < compat_data.len() && compat_data[end] != 0 {
                end += 1;
            }
            
            // 如果找到了有效的字符串
            if end > i {
                let slice = &compat_data[i..end];
                if let Ok(s) = str::from_utf8(slice) {
                    // 这里我们需要将字符串存储在静态生命周期中
                    // 由于我们是在解析DTB时调用此函数，我们可以假设数据是静态的
                    compatibles.push(s);
                }
            }
            
            // 移动到下一个字符串（跳过null终止符）
            i = end + 1;
        }
        
        compatibles
    }

    /// 解析字符串属性
    pub fn parse_string_property(&self, string_data: &[u8]) -> Option<&'static str> {
        // 查找字符串结束符
        let mut end = 0;
        while end < string_data.len() && string_data[end] != 0 {
            end += 1;
        }
        
        // 如果找到了有效的字符串
        if end > 0 {
            let slice = &string_data[0..end];
            if let Ok(s) = str::from_utf8(slice) {
                // 返回静态生命周期的字符串引用
                Some(s)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 解析u32属性
    pub fn parse_u32_property(&self, u32_data: &[u8]) -> Option<u32> {
        if u32_data.len() >= 4 {
            Some(self.be32(&u32_data[0..4]))
        } else {
            None
        }
    }

    /// 解析u64属性
    pub fn parse_u64_property(&self, u64_data: &[u8]) -> Option<u64> {
        if u64_data.len() >= 8 {
            Some(self.be64(&u64_data[0..8]))
        } else {
            None
        }
    }
}

// 固定容量数组向量实现
struct ArrayVec<T, const N: usize> {
    data: [Option<T>; N],
    len: usize,
}

impl<T: Clone, const N: usize> ArrayVec<T, N> {
    fn new() -> Self {
        Self {
            data: [const { None }; N],
            len: 0,
        }
    }

    fn push(&mut self, item: T) {
        if self.len < N {
            self.data[self.len] = Some(item);
            self.len += 1;
        }
    }

    fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            self.data[self.len].take()
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn len(&self) -> usize {
        self.len
    }

    fn iter(&self) -> ArrayVecIter<T, N> {
        ArrayVecIter {
            vec: self,
            index: 0,
        }
    }

    fn join(&self, separator: &str) -> String
    where
        T: AsRef<str>,
    {
        let mut result = String::new();
        let mut first = true;
        
        for item in self.iter() {
            if !first {
                result.push_str(separator);
            }
            result.push_str(item.as_ref());
            first = false;
        }
        
        result
    }

    fn to_vec(&self) -> Vec<T> 
    where 
        T: Clone,
    {
        let mut vec = Vec::new();
        for i in 0..self.len {
            if let Some(item) = &self.data[i] {
                vec.push(item.clone());
            }
        }
        vec
    }
}

impl<T, const N: usize> core::ops::Index<usize> for ArrayVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.data[index].as_ref().unwrap()
    }
}

struct ArrayVecIter<'a, T, const N: usize> {
    vec: &'a ArrayVec<T, N>,
    index: usize,
}

impl<'a, T, const N: usize> Iterator for ArrayVecIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.len {
            let item = self.vec.data[self.index].as_ref().unwrap();
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}