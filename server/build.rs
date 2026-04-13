fn main() {
    println!("🔨 构建脚本执行中...");
    
    #[cfg(windows)]
    {
        extern crate winres;
        
        let mut res = winres::WindowsResource::new();
        
        // 设置图标
        res.set_icon("config/icon.ico");
        println!("✅ 设置图标: config/icon.ico");
        
        // 编译资源
        res.compile().expect("无法编译Windows资源");
        println!("✅ 资源编译成功");
    }
    
    #[cfg(not(windows))]
    {
        println!("ℹ️  非Windows平台，跳过资源编译");
    }
}
