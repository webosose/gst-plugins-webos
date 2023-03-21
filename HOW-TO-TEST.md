## 1. HOW TO BUILD
```bash
TEMP_GSTBUILD=${HOME}/.gsttest
export LD_LIBRARY_PATH=${TEMP_GSTBUILD}/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH
export LD_RUN_PATH=${TEMP_GSTBUILD}/lib/x86_64-linux-gnu:$LD_RUN_PATH
export PKG_CONFIG_PATH=${TEMP_GSTBUILD}/lib/x86_64-linux-gnu/pkgconfig
ORIG_PATH=$PATH
export PATH=${TEMP_GSTBUILD}/bin:${PATH}

rm -rf build
meson build -Dprefix=${TEMP_GSTBUILD} \
    -Ddoc=disabled -Dintrospection=disabled -Dorc=disabled -Dexamples=disabled \
    -Dmachine=$MACHINE
ninja -C build
meson install -C build
```

## 2. HOW TO RUN
```bash
export USE_DECPROXY=1
meson test -C build
PATH=$ORIG_PATH
```

## 3. Information for Test Code

### 3-1. Test Type
Unit Test

### 3-2. Test Code Location
```
./tests/check
```

### 3-3. Production Code Location
```

```

### 3-4. Production Code Language
C

### 3-5. Test Framework
Check | Unit Testing Framework for C

### 3-6. Test Target
Build Server

### 3-7. Environment
```
Dependency: gstreamer, gst-plugin-base

Since there is a dependency between each module of Gstreamer and Plugins,
some modules require installation and settings as temporary paths according to the guide.
When all tests with dependencies are finished, you can remove the temporary working folder.

rm -rf ${TEMP_GSTBUILD}

```
