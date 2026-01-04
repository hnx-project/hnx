project('hnx', 'rust',
  version : run_command('poetry', 'run', 'python', meson.project_source_root() / 'scripts/version.py', 'read', check: true).stdout().strip(),
  meson_version : '>= 1.2.0')

add_project_arguments('--color=always', language : 'rust')

# Get options
arch = get_option('ARCH')
board = get_option('BOARD')
profile = get_option('PROFILE')
rust_profile = profile == 'release' ? 'release' : 'dev'

# Versioning
version_py = meson.project_source_root() / 'scripts/version.py'
version_full = run_command('poetry', 'run', 'python', version_py, 'read', '--full', check: true).stdout().strip()
git_hash = run_command('git', 'rev-parse', '--short', 'HEAD', check: true).stdout().strip()
build_date = run_command('date', '-u', '+%Y%m%d', check: true).stdout().strip()

message('--- HNX Version Information ---')
message('Version: ' + meson.project_version())
message('Full version: ' + version_full)
message('Git commit: ' + git_hash)
message('Build date: ' + build_date)
message('Architecture: ' + arch)
message('Board: ' + board)
message('Profile: ' + profile)
message('Build directory: ' + meson.current_build_dir())
message('-----------------------------')

# Configure script
configure_py = meson.project_source_root() / 'scripts/configure.py'
configure_cmd = [
    'poetry', 'run', 'python', configure_py,
    '--arch', arch,
    '--board', board,
    '--profile', profile,
    '--output-dir', meson.current_build_dir() / 'config'
]
run_command(configure_cmd, check: true)

# Kernel Target
if arch == 'riscv64'
  kernel_target = 'riscv64gc-unknown-none-elf'
elif arch == 'x86_64'
  kernel_target = 'x86_64-unknown-none'
else
  kernel_target = arch + '-unknown-none'
endif

# Space Target
if arch == 'riscv64'
    space_target = 'riscv64gc-unknown-none-elf'
elif arch == 'x86_64'
    space_target = 'x86_64-unknown-none'
else
    space_target = arch + '-unknown-none'
endif

# Version sync
version_sync_cmd = ['poetry', 'run', 'python', version_py, 'sync']
run_command(version_sync_cmd, check: true)

# Kernel build
kernel_build = custom_target('kernel',
  input : 'kernel/Cargo.toml',
  output : 'hnx-kernel',
  command : [
    'cargo', 'build',
    '-p', 'hnx-kernel',
    '--manifest-path', '@INPUT@',
    '--target', kernel_target,
    '--profile', rust_profile,
    '--target-dir', meson.current_build_dir()+'/kernel'
  ],
  env : ['RUSTFLAGS=-A warnings'],
  build_by_default : true,
  console: true
)

# Space build
space_build = custom_target('space',
  input : 'space/Cargo.toml',
  output : 'space_components',
  command : [
    'cargo', 'build',
    '--workspace', '--manifest-path', '@INPUT@', '--target', space_target, '--profile', rust_profile, '--target-dir', meson.current_build_dir()+'/space'
  ],
  env : ['RUSTFLAGS=-A warnings'],
  depends: [kernel_build],
  build_by_default : true,
  console: true
)

# Image build
image_build = custom_target('image',
  input : meson.project_source_root() / 'scripts/create-image.py',
  output : 'hnx.img',
  command : [
    'poetry', 'run', 'python', '@INPUT@',
    '--kernel', meson.current_build_dir() / 'kernel' / kernel_target / profile / 'hnx-kernel',
    '--space-dir', meson.current_build_dir() / 'space' / space_target / profile,
    '--arch', arch,
    '--board', board,
    '--no-compress',
    '--output', meson.current_build_dir() / 'images' / 'hnx.img'
  ],
  depends: [space_build],
  build_by_default : true,
  console: true
)

# Simple image build
simple_image_build = custom_target('simple-image',
    input : meson.project_source_root() / 'scripts/create-image.py',
    output : 'hnx-simple.img',
    command : [
        'poetry', 'run', 'python', '@INPUT@',
        '--kernel', meson.current_build_dir() / 'kernel' / kernel_target / profile / 'hnx-kernel',
        '--space-dir', meson.current_build_dir() / 'space' / space_target / profile,
        '--arch', arch,
        '--board', board,
        '--simple-initrd',
        '--no-compress',
        '--output', meson.current_build_dir() / 'images' / 'hnx-simple.img'
    ],
    depends: [space_build],
    build_by_default : true,
    console: true
)

# Run target
run_qemu_py = meson.project_source_root() / 'scripts/run-qemu.py'
run_target('run',
    command: [
        'poetry', 'run', 'python', run_qemu_py,
        '--arch', arch,
        '--board', board,
        '--headless',
        '--config-dir', meson.current_build_dir() / 'config',
        meson.current_build_dir() / 'images' / 'hnx.img'
    ],
    depends: [image_build]
)

# Run simple target
run_target('run-simple',
    command: [
        'poetry', 'run', 'python', run_qemu_py,
        '--arch', arch,
        '--board', board,
        '--headless',
        '--timeout', '30',
        meson.current_build_dir() / 'kernel' / kernel_target / profile / 'hnx-kernel'
    ],
    depends: [simple_image_build]
)

# Debug target
run_target('debug',
    command: [
        'poetry', 'run', 'python', run_qemu_py,
        '--arch', arch,
        '--board', board,
        '--headless',
        '--gdb',
        '--timeout', '0',
        meson.current_build_dir() / 'images' / 'hnx.img'
    ],
    depends: [image_build]
)

# Test target
test('kernel-test',
  find_program('cargo'),
  args: ['test', '--manifest-path', meson.project_source_root() / 'kernel/Cargo.toml', '--lib'],
  workdir: meson.project_source_root() / 'kernel',
)
test('space-test',
    find_program('cargo'),
    args: ['test', '--manifest-path', meson.project_source_root() / 'space/Cargo.toml', '--workspace'],
    workdir: meson.project_source_root() / 'space',
)