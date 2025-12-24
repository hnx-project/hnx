//! 设备树(blob)解析模块
//!
//! 提供对设备树二进制格式(DTB)的完整解析能力，包括：
//! - 设备树节点遍历
//! - 属性解析
//! - 设备匹配
//! - 设备查找

use core::str;

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
#[derive(Debug, Clone, Copy)]
pub struct DtbProperty {
    /// 属性名称
    pub name: &'static str,
    /// 属性值
    pub value: &'static [u8],
}

/// 设备匹配条件
pub struct DeviceMatch {
    /// 兼容性字符串
    pub compatible: Option<&'static str>,
    /// 设备类型
    pub device_type: Option<&'static str>,
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
                        
                        let node = DtbNode {
                            name,
                            path: name, // 简化实现，实际应该构建完整路径
                            offset: name_start,
                        };
                        
                        // 解析属性
                        let properties = self.parse_properties(&mut i);
                        // 创建一个临时数组来存储属性
                        let mut prop_array: [DtbProperty; 16] = [DtbProperty { name: "", value: &[] }; 16];
                        let mut prop_count = 0;
                        for i in 0..properties.len {
                            if let Some(prop) = &properties.data[i] {
                                if prop_count < 16 {
                                    prop_array[prop_count] = prop.clone();
                                    prop_count += 1;
                                }
                            }
                        }
                        callback(&node, &prop_array[..prop_count]);
                    } else {
                        // 对齐到4字节边界
                        while i % 4 != 0 {
                            i += 1;
                        }
                    }
                }
                
                // FDT_END_NODE
                2 => {
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

    /// 检查节点是否匹配给定条件
    fn node_matches(&self, _node: &DtbNode, properties: &[DtbProperty], match_criteria: &DeviceMatch) -> bool {
        // 检查兼容性字符串
        if let Some(compat) = match_criteria.compatible {
            if let Some(compat_prop) = properties.iter().find(|p| p.name == "compatible") {
                if let Ok(compat_str) = str::from_utf8(compat_prop.value) {
                    if !compat_str.contains(compat) {
                        return false;
                    }
                } else {
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
        
        true
    }
}

/// 固定容量数组向量实现
pub struct ArrayVec<T, const N: usize> {
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

    fn iter(&self) -> ArrayVecIter<'_, T, N> {
        ArrayVecIter {
            vec: self,
            index: 0,
        }
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