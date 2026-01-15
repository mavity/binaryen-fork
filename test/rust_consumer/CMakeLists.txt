if(BUILD_RUST_COMPONENTS)

  # Simple smoke consumer that links against the Rust-generated library
  add_executable(test_rust_consumer
    test_ffi.c
  )

  add_executable(test_rust_consumer_threaded
    test_ffi_threaded.cpp
  )

  add_executable(test_rust_consumer_arena_threads
    test_ffi_arena_threads.cpp
  )

  add_executable(test_rust_consumer_arena_misuse
    test_ffi_arena_misuse.cpp
  )

  add_executable(test_rust_consumer_arena_deref_after_dispose
    test_ffi_arena_deref_after_dispose.cpp
  )

  add_executable(test_rust_consumer_arena_race_dispose
    test_ffi_arena_race_dispose.cpp
  )

  add_executable(test_rust_consumer_arena_handle
    test_ffi_arena_handle.cpp
  )

  add_executable(test_rust_consumer_arena_use_after_dispose
    test_ffi_arena_use_after_dispose.cpp
  )

  add_executable(test_rust_consumer_arena_many_threads
    test_ffi_arena_many_threads.cpp
  )

  add_executable(test_rust_consumer_extra
    test_ffi_extra.c
  )

  add_executable(test_rust_consumer_type_roundtrip
    test_ffi_type_roundtrip.c
  )

  add_executable(test_rust_consumer_type_roundtrip_cpp
    test_ffi_type_roundtrip.cpp
  )

  add_executable(test_rust_consumer_ir
    test_ffi_ir.cpp
  )

  add_executable(test_rust_binary_io
    test_rust_binary_io.cpp
  )

  add_executable(pipeline_demo
    pipeline_demo.cpp
  )

  # Some build environments treat the global warning flags as C++-only
  # and will error when compiling .c files. Build this test as C++ to avoid
  # those flags and allow linking against the Rust cdylib. The header is
  # `extern "C"` so it's safe to compile as C++.
  set_source_files_properties(test_ffi.c PROPERTIES LANGUAGE CXX)
  # Some test C files require C++ flags; compile as C++ to avoid C-only warning errors
  set_source_files_properties(test_ffi_extra.c PROPERTIES LANGUAGE CXX)
  set_source_files_properties(test_ffi_type_roundtrip.c PROPERTIES LANGUAGE CXX)

  # Link against the imported Rust static library if available, otherwise use
  # the path in ${RUST_LIBS} if that variable is set. Also link platform
  # libs that the consumer expects.
  if(TARGET binaryen_ffi_static)
    # Link all rust consumer tests against the imported Rust static library
    target_link_libraries(test_rust_consumer PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_threaded PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_threads PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_misuse PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_deref_after_dispose PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_race_dispose PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_handle PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_use_after_dispose PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_arena_many_threads PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_extra PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_type_roundtrip PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_type_roundtrip_cpp PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_consumer_ir PRIVATE binaryen_ffi_static)
    target_link_libraries(test_rust_binary_io PRIVATE binaryen_ffi_static)
    target_link_libraries(pipeline_demo PRIVATE binaryen_ffi_static)
  elseif(RUST_LIBS)
    target_link_libraries(test_rust_consumer PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_threaded PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_threads PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_misuse PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_deref_after_dispose PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_race_dispose PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_handle PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_use_after_dispose PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_arena_many_threads PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_extra PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_type_roundtrip PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_type_roundtrip_cpp PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_consumer_ir PRIVATE ${RUST_LIBS})
    target_link_libraries(test_rust_binary_io PRIVATE ${RUST_LIBS})
    target_link_libraries(pipeline_demo PRIVATE ${RUST_LIBS})
  endif()

  # Link with dl and pthread if required; add Threads dependency to be safe.
  find_package(Threads REQUIRED)
  target_link_libraries(test_rust_consumer PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)

  target_link_libraries(test_rust_consumer_extra PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_type_roundtrip PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_type_roundtrip_cpp PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_ir PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_binary_io PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(pipeline_demo PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)

  target_link_libraries(test_rust_consumer_threaded PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_threads PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_misuse PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_deref_after_dispose PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_race_dispose PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_handle PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_use_after_dispose PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)
  target_link_libraries(test_rust_consumer_arena_many_threads PRIVATE ${CMAKE_DL_LIBS} Threads::Threads)

  # Ensure the Rust artifacts are built before the test runner is linked
  if(TARGET rust_libs)
    add_dependencies(test_rust_consumer rust_libs)
    add_dependencies(test_rust_consumer_threaded rust_libs)
    add_dependencies(test_rust_consumer_arena_threads rust_libs)
  endif()

  # Run as a test so `ctest` can run it during CI without touching external
  # build scripts.
  enable_testing()
  add_test(NAME rust_ffi_smoke COMMAND $<TARGET_FILE:test_rust_consumer>)
  add_test(NAME rust_ffi_smoke_extra COMMAND $<TARGET_FILE:test_rust_consumer_extra>)
  add_test(NAME rust_ffi_smoke_type_roundtrip COMMAND $<TARGET_FILE:test_rust_consumer_type_roundtrip>)
  add_test(NAME rust_ffi_smoke_type_roundtrip_cpp COMMAND $<TARGET_FILE:test_rust_consumer_type_roundtrip_cpp>)
  add_test(NAME rust_ffi_smoke_ir COMMAND $<TARGET_FILE:test_rust_consumer_ir>)
  add_test(NAME rust_binary_io COMMAND $<TARGET_FILE:test_rust_binary_io>)
  add_test(NAME rust_pipeline_demo COMMAND $<TARGET_FILE:pipeline_demo>)
  add_test(NAME rust_ffi_smoke_threaded COMMAND $<TARGET_FILE:test_rust_consumer_threaded>)
  add_test(NAME rust_ffi_smoke_arena_threads COMMAND $<TARGET_FILE:test_rust_consumer_arena_threads>)
  add_test(NAME rust_ffi_smoke_arena_misuse COMMAND $<TARGET_FILE:test_rust_consumer_arena_misuse>)
  add_test(NAME rust_ffi_smoke_arena_deref_after_dispose COMMAND $<TARGET_FILE:test_rust_consumer_arena_deref_after_dispose>)
  add_test(NAME rust_ffi_smoke_arena_race_dispose COMMAND $<TARGET_FILE:test_rust_consumer_arena_race_dispose>)
  add_test(NAME rust_ffi_smoke_arena_handle COMMAND $<TARGET_FILE:test_rust_consumer_arena_handle>)
  add_test(NAME rust_ffi_smoke_arena_use_after_dispose COMMAND $<TARGET_FILE:test_rust_consumer_arena_use_after_dispose>)
  add_test(NAME rust_ffi_smoke_arena_many_threads COMMAND $<TARGET_FILE:test_rust_consumer_arena_many_threads>)

endif()
