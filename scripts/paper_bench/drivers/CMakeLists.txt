# SoPlex ドライバのビルド (setup_solvers.sh から呼ぶ)。SoPlex のインストール先を
# CMAKE_PREFIX_PATH に渡すこと。
cmake_minimum_required(VERSION 3.16)
project(paper_bench_drivers CXX)
set(CMAKE_CXX_STANDARD 17)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

find_package(SOPLEX REQUIRED CONFIG)
set(SOPLEX_VERSION_STR "unknown" CACHE STRING "SoPlex version recorded in the benchmark output")

add_executable(soplex_driver soplex_driver.cpp)
target_include_directories(soplex_driver PRIVATE ${SOPLEX_INCLUDE_DIRS})
target_link_libraries(soplex_driver PRIVATE ${SOPLEX_LIBRARIES})
target_compile_definitions(soplex_driver PRIVATE SOPLEX_VERSION_STR="${SOPLEX_VERSION_STR}")
