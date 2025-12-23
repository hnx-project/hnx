//! Memory allocator tests

#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    
    #[test]
    fn test_basic_allocation() {
        unsafe {
            let layout = Layout::from_size_align(1024, 4).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            assert!(!ptr.is_null());
            
            // Write some data to the allocated memory
            for i in 0..1024 {
                *ptr.add(i) = i as u8;
            }
            
            // Verify the data
            for i in 0..1024 {
                assert_eq!(*ptr.add(i), i as u8);
            }
            
            // Deallocate the memory
            alloc::alloc::dealloc(ptr, layout);
        }
    }
    
    #[test]
    fn test_multiple_allocations() {
        unsafe {
            let layout1 = Layout::from_size_align(256, 4).unwrap();
            let layout2 = Layout::from_size_align(512, 4).unwrap();
            let layout3 = Layout::from_size_align(1024, 4).unwrap();
            
            let ptr1 = alloc::alloc::alloc(layout1);
            let ptr2 = alloc::alloc::alloc(layout2);
            let ptr3 = alloc::alloc::alloc(layout3);
            
            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());
            
            // Verify that the pointers are different
            assert_ne!(ptr1, ptr2);
            assert_ne!(ptr1, ptr3);
            assert_ne!(ptr2, ptr3);
            
            // Deallocate the memory
            alloc::alloc::dealloc(ptr1, layout1);
            alloc::alloc::dealloc(ptr2, layout2);
            alloc::alloc::dealloc(ptr3, layout3);
        }
    }
    
    #[test]
    fn test_allocation_alignment() {
        unsafe {
            let layout = Layout::from_size_align(100, 16).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            assert!(!ptr.is_null());
            
            // Check alignment
            assert_eq!((ptr as usize) % 16, 0);
            
            // Deallocate the memory
            alloc::alloc::dealloc(ptr, layout);
        }
    }
    
    #[test]
    fn test_zero_sized_allocation() {
        unsafe {
            let layout = Layout::from_size_align(0, 1).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            // Zero-sized allocations may return null or a valid pointer
            // We just check that it doesn't crash
            alloc::alloc::dealloc(ptr, layout);
        }
    }
}