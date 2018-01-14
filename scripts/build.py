import argparse
import os
import sh
import shutil
import toml
import zipfile

APP_NAME = "punchcards"
README_NAME = "README.md"
LICENSE_NAME = "LICENSE"

BINARY_NAMES = { "unix": "%s_unix" % APP_NAME,
                 "glutin": "%s_glutin" % APP_NAME,
               }

OUTPUT_PREFIXES = { "unix": "%s-terminal" % APP_NAME,
                    "glutin": "%s-opengl" % APP_NAME,
                  }

SH_KWARGS = { "_out": "/dev/stdout",
              "_err": "/dev/stderr",
              "_fg": True,
            }

def make_parser():
  parser = argparse.ArgumentParser()
  parser.add_argument('--frontend')
  parser.add_argument('--build-path')
  parser.add_argument('--upload-path')
  parser.add_argument('--crate-path')
  parser.add_argument('--root-path')
  parser.add_argument('--os')
  return parser

def build_common(args):
  args.architecture = "x86_64"
  sh.cargo.build(
    "--manifest-path", args.manifest_path,
    "--release",
    **SH_KWARGS)
  output_dir_name = "%(prefix)s-%(os)s-%(architecture)s-v%(version)s" % {
      "prefix": OUTPUT_PREFIXES[args.frontend],
      "os": args.os,
      "architecture": args.architecture,
      "version": args.version,
  }
  output_dir_path = os.path.join(args.build_path, output_dir_name)
  os.makedirs(output_dir_path)
  shutil.copy(
      os.path.join(args.crate_path, "target", "release", BINARY_NAMES[args.frontend]),
      os.path.join(output_dir_path, APP_NAME))
  shutil.copy(
      os.path.join(args.root_path, README_NAME),
      os.path.join(output_dir_path, README_NAME))
  shutil.copy(
      os.path.join(args.root_path, LICENSE_NAME),
      os.path.join(output_dir_path, LICENSE_NAME))
  sh.git("rev-parse", "HEAD", _err="/dev/stderr",
      _out=os.path.join(output_dir_path, "revision.txt"))

  if not os.path.exists(args.upload_path):
    os.makedirs(args.upload_path)

  zip_name = "%s.zip" % output_dir_name
  zip_path = os.path.join(args.upload_path, zip_name)

  with zipfile.ZipFile(zip_path, 'w') as zip_file:
    for subdir, dirs, files in os.walk(output_dir_path):
      for f in files:
        arcname = os.path.join(os.path.basename(subdir), f)
        zip_file.write(os.path.join(subdir, f), arcname)

  if args.os == "macos":
    args.output_dir_path = output_dir_path
    make_macos_app(args)

def make_macos_app(args):
  pass

def build_wasm(args):
  crate_path = os.path.normpath(args.crate_path)
  sh.bash(os.path.join(crate_path, "build.sh"), "--with-npm-install",
    **SH_KWARGS)

  output_dir_path = os.path.join(args.upload_path, APP_NAME)
  os.makedirs(output_dir_path)
  shutil.copytree(
      os.path.join(args.crate_path, "dist"),
      os.path.join(output_dir_path, "v%s" % args.version))

BUILD_FNS = { "unix": build_common,
              "glutin": build_common,
              "wasm": build_wasm,
            }

def main(args):
  args.crate_path = os.path.normpath(args.crate_path)
  args.build_path = os.path.normpath(args.build_path)
  args.upload_path = os.path.normpath(args.upload_path)
  args.root_path = os.path.normpath(args.root_path)
  args.manifest_path = os.path.join(args.crate_path, "Cargo.toml")
  args.manifest = toml.load(args.manifest_path)
  args.version = args.manifest['package']['version']
  BUILD_FNS[args.frontend](args)

if __name__ == "__main__":
  parser = make_parser()
  main(parser.parse_args())