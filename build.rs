use std::{env, process::Command};

macro_rules! str {
    ($pathBuf:tt) => {
        $pathBuf.clone().to_str().unwrap()
    };
}
macro_rules! safe_path {
    ($pathBuf:tt) => {
        $pathBuf.clone().to_str().unwrap().replace("\\", "/")
    };
    ($pathBuf:tt, $joinPath:tt) => {
        $pathBuf.clone().join($joinPath).to_str().unwrap().replace("\\", "/")
    };
}

fn main() {
    let vcpkg_root = env::current_dir().unwrap().join("vcpkg");
    let vcpkg_toolchain = vcpkg_root.clone().join("scripts/buildsystems/vcpkg.cmake");
    // TODO get from env
    let vcpkg_triplet_root = vcpkg_root.clone().join("installed/x64-windows-static-md");

    env::set_var("VCPKG_ROOT", str!(vcpkg_root));
    // unofficial-brotli とかになるので無理
    env::set_var("VCPKG_NO_BROTLI", "1");

    // Build BoringSSL
    let ssl_dst = cmake::Config::new("deps/boringssl")
        .define("CMAKE_TOOLCHAIN_FILE", str!(vcpkg_toolchain))
        .define("CMAKE_BUILD_TYPE", "Release")
        .generator("Ninja")
        .build();
    println!("cargo:rustc-link-search=native={}", ssl_dst.clone().join("lib").display());
    println!("cargo:rustc-link-lib=static=ssl"); 
    println!("cargo:rustc-link-lib=static=crypto"); 

    // Build Brotli
    let brotli_dst = cmake::Config::new("deps/brotli")
        .define("CMAKE_TOOLCHAIN_FILE", str!(vcpkg_toolchain))
        .define("CMAKE_BUILD_TYPE", "Release")
        .build();
    println!("cargo:rustc-link-search=native={}", brotli_dst.clone().join("lib").display());
    println!("cargo:rustc-link-lib=static=brotlicommon"); 
    println!("cargo:rustc-link-lib=static=brotlienc"); 
    println!("cargo:rustc-link-lib=static=brotlidec"); 

    // Build Quiche
    let quiche_out = Command::new("cargo")
        .current_dir("deps/quiche")
        .env("QUICHE_BSSL_PATH", ssl_dst.clone().join("lib").to_str().unwrap())
        .args(["build", "--package", "quiche", "--release", "-vv", "--features", "ffi,pkg-config-meta,qlog"])
        .output()
        .expect("failed to build quiche");
    println!("{}", String::from_utf8(quiche_out.stdout).unwrap());
    if quiche_out.status.success() {
        println!(":: Quiche has been built.")
    } else {
        eprintln!("{}", String::from_utf8(quiche_out.stderr).unwrap());
        panic!("Quiche build failed")
    }
    let quiche_dst = env::current_dir().unwrap().join("deps/quiche/target/release");
    let quiche_include = env::current_dir().unwrap().join("deps/quiche/quiche/include");
    println!("cargo:rustc-link-search=native={}", str!(quiche_dst));
    println!("cargo:rustc-link-lib=static=quiche"); 

    // Build Curl
    let curl_dst = cmake::Config::new("deps/curl")
        .env("LDFLAGS", "ntdll.lib userenv.lib")
        .define("CMAKE_TOOLCHAIN_FILE", str!(vcpkg_toolchain))
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("BUILD_SHARED_LIBS", "NO")
        // OpenSSL (BoringSSL)
        .define("CURL_USE_OPENSSL", "YES")
        .define("OPENSSL_ROOT_DIR", ssl_dst.clone().to_str().unwrap())
        // NgHTTP2 (HTTP/2)
        .define("USE_NGHTTP2", "YES")
        .define("NGHTTP2_INCLUDE_DIR", safe_path!(vcpkg_triplet_root, "include"))
        .define("NGHTTP2_LIBRARY", safe_path!(vcpkg_triplet_root, "lib/nghttp2.lib"))
        // quiche
        .define("USE_QUICHE", "YES")
        .define("QUICHE_INCLUDE_DIR", str!(quiche_include))
        .define("QUICHE_LIBRARY", quiche_dst.join("quiche.lib").to_str().unwrap())
        // 一般的なライブラリ
        .define("ZLIB_INCLUDE_DIR", safe_path!(vcpkg_triplet_root, "include"))
        .define("ZLIB_LIBRARY", safe_path!(vcpkg_triplet_root, "lib/zlib.lib"))
        .define("CURL_BROTLI", "YES")
        .define("BROTLI_INCLUDE_DIRS", brotli_dst.clone().join("include").to_str().unwrap())
        .define("BROTLI_LIBRARIES", brotli_dst.clone().join("lib").to_str().unwrap())
        // 最低限でいいので除外
        .define("HTTP_ONLY", "YES")
        .define("CURL_DISABLE_NTLM", "YES")
        // Build
        .generator("Ninja")
        .build();
    println!("cargo:rustc-link-search=native={}", curl_dst.clone().join("lib").display());
    println!("cargo:rustc-link-lib=static=libcurl"); 
    // println!("cargo:rustc-link-lib=dynamic=msvcrt");
    // println!("cargo:rustc-link-lib=static=ws2_32");
    // println!("cargo:rustc-link-lib=static=userenv");
}
