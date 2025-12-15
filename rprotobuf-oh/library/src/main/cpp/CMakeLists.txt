cmake_minimum_required(VERSION 3.4.1)
project(rprotobuf)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})

include_directories(${NATIVERENDER_ROOT_PATH}/include)

# Note: This is a placeholder. To build Rust code in OpenHarmony, 
# you typically need to use a toolchain that supports Rust or 
# pre-build the Rust code into a static library and link it here.
# 
# Example for linking a pre-built Rust static library:
# add_library(rust_lib STATIC IMPORTED)
# set_target_properties(rust_lib PROPERTIES IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target/aarch64-linux-ohos/release/librprotobuf.a)

add_library(rprotobuf SHARED hello.cpp)
# target_link_libraries(rprotobuf PUBLIC rust_lib)
