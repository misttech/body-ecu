# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

# rivet as the C library of a Zephyr application built with CONFIG_EXTERNAL_LIBC. A module,
# not the application, sets it: Zephyr fixes its link command while it processes its own tree,
# before the rest of the application's CMakeLists.txt runs, and modules are processed inside
# that tree. RIVET_INCLUDE_DIR is rivet's include/; RIVET_LIBRARY its archive, built for the
# board's Rust target with the cmpctmalloc feature. The archive takes the place of -lc; it
# carries the compiler runtime, so the -lgcc Zephyr still adds after it supplies nothing.
if(CONFIG_EXTERNAL_LIBC)
  if(NOT RIVET_INCLUDE_DIR OR NOT RIVET_LIBRARY)
    message(FATAL_ERROR "rivet module: set RIVET_INCLUDE_DIR and RIVET_LIBRARY")
  endif()
  zephyr_system_include_directories(${RIVET_INCLUDE_DIR})
  set_property(TARGET linker PROPERTY c_library ${RIVET_LIBRARY})
endif()
