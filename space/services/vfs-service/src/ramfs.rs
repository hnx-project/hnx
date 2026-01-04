use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;

#[derive(Clone, Debug)]
pub enum Node {
    File { data: Vec<u8> },
    Dir { children: BTreeMap<String, usize> },
}

pub struct RamFs {
    nodes: Mutex<Vec<Node>>,
    next_inode: Mutex<usize>,
}

impl RamFs {
    pub fn new() -> Self {
        let mut nodes = Vec::new();
        nodes.push(Node::Dir { children: BTreeMap::new() });
        
        Self {
            nodes: Mutex::new(nodes),
            next_inode: Mutex::new(1),
        }
    }
    
    pub fn mkdir(&mut self, path: &str) -> Result<usize, ()> {
        let mut nodes = self.nodes.lock();
        let mut next_inode = self.next_inode.lock();
        
        if path == "/" {
            return Ok(0);
        }
        
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let mut current = 0usize;
        
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            
            match &nodes[current] {
                Node::Dir { children } => {
                    if let Some(&child_inode) = children.get(*part) {
                        current = child_inode;
                    } else if is_last {
                        let new_inode = *next_inode;
                        *next_inode += 1;
                        nodes.push(Node::Dir { children: BTreeMap::new() });
                        if let Node::Dir { children } = &mut nodes[current] {
                            children.insert(String::from(*part), new_inode);
                        }
                        return Ok(new_inode);
                    } else {
                        return Err(());
                    }
                }
                Node::File { .. } => return Err(()),
            }
        }
        
        Ok(current)
    }
    
    pub fn create(&mut self, path: &str) -> Result<usize, ()> {
        let mut nodes = self.nodes.lock();
        let mut next_inode = self.next_inode.lock();
        
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        if parts.is_empty() {
            return Err(());
        }
        
        let mut current = 0usize;
        
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            
            if is_last {
                match &nodes[current] {
                    Node::Dir { children } => {
                        if children.contains_key(*part) {
                            return Err(());
                        }
                        let new_inode = *next_inode;
                        *next_inode += 1;
                        nodes.push(Node::File { data: Vec::new() });
                        if let Node::Dir { children } = &mut nodes[current] {
                            children.insert(String::from(*part), new_inode);
                        }
                        return Ok(new_inode);
                    }
                    Node::File { .. } => return Err(()),
                }
            } else {
                match &nodes[current] {
                    Node::Dir { children } => {
                        if let Some(&child_inode) = children.get(*part) {
                            current = child_inode;
                        } else {
                            return Err(());
                        }
                    }
                    Node::File { .. } => return Err(()),
                }
            }
        }
        
        Err(())
    }
    
    pub fn read(&self, path: &str, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let nodes = self.nodes.lock();
        let inode = self.lookup(&nodes, path)?;
        
        match &nodes[inode] {
            Node::File { data } => {
                if offset >= data.len() {
                    return Ok(0);
                }
                let to_read = core::cmp::min(buf.len(), data.len() - offset);
                buf[..to_read].copy_from_slice(&data[offset..offset + to_read]);
                Ok(to_read)
            }
            Node::Dir { .. } => Err(()),
        }
    }
    
    pub fn write(&mut self, path: &str, offset: usize, data: &[u8]) -> Result<usize, ()> {
        let mut nodes = self.nodes.lock();
        let inode = self.lookup(&nodes, path)?;
        
        match &mut nodes[inode] {
            Node::File { data: file_data } => {
                if offset > file_data.len() {
                    file_data.resize(offset, 0);
                }
                
                let end = offset + data.len();
                if end > file_data.len() {
                    file_data.resize(end, 0);
                }
                
                file_data[offset..end].copy_from_slice(data);
                Ok(data.len())
            }
            Node::Dir { .. } => Err(()),
        }
    }
    
    pub fn unlink(&mut self, path: &str) -> Result<(), ()> {
        let mut nodes = self.nodes.lock();
        
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        if parts.is_empty() {
            return Err(());
        }
        
        let mut current = 0usize;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                match &mut nodes[current] {
                    Node::Dir { children } => {
                        if children.remove(*part).is_some() {
                            return Ok(());
                        } else {
                            return Err(());
                        }
                    }
                    Node::File { .. } => return Err(()),
                }
            } else {
                match &nodes[current] {
                    Node::Dir { children } => {
                        if let Some(&child_inode) = children.get(*part) {
                            current = child_inode;
                        } else {
                            return Err(());
                        }
                    }
                    Node::File { .. } => return Err(()),
                }
            }
        }
        
        Err(())
    }
    
    fn lookup(&self, nodes: &Vec<Node>, path: &str) -> Result<usize, ()> {
        if path == "/" {
            return Ok(0);
        }
        
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let mut current = 0usize;
        
        for part in parts {
            match &nodes[current] {
                Node::Dir { children } => {
                    if let Some(&child_inode) = children.get(part) {
                        current = child_inode;
                    } else {
                        return Err(());
                    }
                }
                Node::File { .. } => return Err(()),
            }
        }
        
        Ok(current)
    }
}