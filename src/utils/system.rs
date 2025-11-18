use libc::*;

//CPU 코어 개수 조회
pub fn get_cpu_count() -> usize {
    let count: i64 = unsafe { sysconf(_SC_NPROCESSORS_ONLN) };
    if count > 0 { count as usize } else { 4 }
}
