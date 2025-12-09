//! A build dependency for compiling Java source code, similar to the `cc` crate for C/C++.

//!
//! # Example
//!
//! ```no_run
//! # fn main() -> Result<(), javac::Error> {
//! javac::Build::new()
//!     .file("tests/java/com/example/Foo.java")
//!     .file("tests/java/com/example/Bar.java")
//!     .compile();
//! # Ok(())
//! # }
//! ```

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::FromUtf8Error;

/// Result type alias using the custom [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for javac operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error occurred during compilation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 conversion error when processing javac output.
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] FromUtf8Error),

    /// Javac compiler not found.
    #[error("javac compiler not found: {0}")]
    CompilerNotFound(String),

    /// No source files specified.
    #[error("No source files specified for compilation")]
    NoSourceFiles,

    /// Compilation failed.
    #[error("javac compilation failed:\nstdout: {stdout}\nstderr: {stderr}")]
    CompilationFailed { stdout: String, stderr: String },

    /// No class files were generated.
    #[error(
        "javac compilation succeeded but no [wrote ...] lines were found in stderr.\nThis may indicate javac is not outputting verbose messages.\nstdout: {stdout}\nstderr: {stderr}"
    )]
    NoClassFilesGenerated { stdout: String, stderr: String },

    /// Invalid classpath.
    #[error("invalid classpath: {0}")]
    InvalidClasspath(String),

    /// Invalid directory.
    #[error("not a directory: {0}")]
    InvalidDirectory(String),

    /// Environment variable not found.
    #[error("OUT_DIR environment variable not set and no output directory specified")]
    OutDirNotSet,

    /// Unsupported feature for the current Java version.
    #[error("Unsupported: {0}")]
    Unsupported(String),
}

/// A builder for compiling Java source files.
///
/// This struct follows a builder pattern similar to `cc::Build`, allowing you to
/// configure compilation options before invoking the Java compiler.
///
/// # Example
///
/// ```no_run
/// # fn main() -> Result<(), javac::Error> {
/// javac::Build::new()
///     .file("src/Foo.java")
///     .file("src/Bar.java")
///     .source_version("11")
///     .target_version("11")
///     .compile();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Build {
    files: Vec<PathBuf>,
    source_dirs: Vec<PathBuf>,
    classpath: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    output_src_dir: Option<PathBuf>,
    output_subdir: Option<PathBuf>,
    source_version: Option<String>,
    target_version: Option<String>,
    release: Option<String>,
    encoding: Option<String>,
    debug: bool,
    warnings: bool,
    werror: bool,
    emit_rerun_if_changed: bool,
    cargo_metadata: bool,
    extra_args: Vec<String>,
    // Cached javac version for internal use (not exposed in public API)
    #[allow(dead_code)]
    javac_version: Option<u32>,
}

impl Default for Build {
    fn default() -> Self {
        Self::new()
    }
}

impl Build {
    /// Create a new `Build` instance.
    pub fn new() -> Self {
        Build {
            files: Vec::new(),
            source_dirs: Vec::new(),
            classpath: Vec::new(),
            output_dir: None,
            output_src_dir: None,
            output_subdir: None,
            source_version: None,
            target_version: None,
            release: None,
            encoding: None,
            debug: false,
            warnings: true,
            werror: false,
            emit_rerun_if_changed: false,
            cargo_metadata: true,
            extra_args: Vec::new(),
            javac_version: None,
        }
    }

    /// Add a single source file to be compiled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn file<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.files.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple source files to be compiled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let files = vec!["src/Foo.java", "src/Bar.java"];
    /// javac::Build::new()
    ///     .files(files)
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn files<P: AsRef<Path>>(&mut self, paths: impl IntoIterator<Item = P>) -> &mut Self {
        self.files
            .extend(paths.into_iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Add a directory to be scanned for `.java` files recursively.
    ///
    /// All `.java` files found in the directory and its subdirectories will be compiled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .source_dir("src/main/java")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn source_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.source_dirs.push(path.as_ref().to_path_buf());
        self
    }

    /// Enable printing `cargo:rerun-if-changed=` lines for all source inputs.
    ///
    /// When enabled, the compiler will print lines for all explicit source
    /// files and for each directory added via [`Self::source_dir`]. This lets
    /// Cargo rebuild when any of the known source files change, or when new
    /// `.java` files are added under the watched directories.
    ///
    /// Note: This has no effect unless [`Self::cargo_metadata`] is `true` (the default).
    ///
    /// Defaults to `false`.
    pub fn emit_rerun_if_changed(&mut self, enable: bool) -> &mut Self {
        self.emit_rerun_if_changed = enable;
        self
    }

    /// Control whether to emit any `cargo:` metadata lines (such as rerun-if-changed or rerun-if-env-changed).
    ///
    /// If set to `false`, no `cargo:` lines will be printed, regardless of other settings.
    /// This is useful if you are not running in a Cargo build script context.
    ///
    /// Defaults to `true`.
    pub fn cargo_metadata(&mut self, enable: bool) -> &mut Self {
        self.cargo_metadata = enable;
        self
    }

    /// Add a path to the classpath.
    ///
    /// Multiple classpath entries can be added and will be joined with the
    /// platform-specific path separator.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .classpath("lib/dependency.jar")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn classpath<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.classpath.push(path.as_ref().to_path_buf());
        self
    }

    /// Set the output directory for compiled `.class` files.
    ///
    /// If not set, defaults to `$OUT_DIR/javac-classes/` (with the `OUT_DIR`
    /// environment variable typically set by Cargo).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .output_dir("target/classes")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn output_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.output_dir = Some(path.as_ref().to_path_buf());
        // If output_src_dir is not set, set it to the same as output_dir by default
        if self.output_src_dir.is_none() {
            self.output_src_dir = self.output_dir.clone();
        }
        self
    }

    /// Get the output directory for generated sources, defaulting as described.
    fn get_output_src_dir(&self) -> Result<PathBuf> {
        if let Some(ref dir) = self.output_src_dir {
            return Ok(dir.clone());
        }
        // If output_dir is set, default to that
        if let Some(ref dir) = self.output_dir {
            return Ok(dir.clone());
        }
        // Otherwise, use $OUT_DIR/javac-build/generated-sources/
        let out_dir = std::env::var("OUT_DIR")
            .map(PathBuf::from)
            .map_err(|_| Error::OutDirNotSet)?;
        Ok(out_dir.join("javac-build/generated-sources/"))
    }

    /// Set a subdirectory within the output directory for compiled `.class` files.
    ///
    /// This subdirectory is appended to the base output directory (either the explicitly
    /// set `output_dir()` or the default `OUT_DIR` from Cargo). This is useful for
    /// separating multiple compilation runs without needing to manually compose paths.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// // Compiles to $OUT_DIR/javac-classes/example/
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .output_subdir("example")
    ///     .compile();
    ///
    /// // Compiles to custom-dir/java-classes/
    /// javac::Build::new()
    ///     .file("src/Bar.java")
    ///     .output_dir("custom-dir")
    ///     .output_subdir("java-classes")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn output_subdir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.output_subdir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the output directory for generated source files (`javac -s`).
    ///
    /// If not set, defaults to `$OUT_DIR/javac-build/generated-sources/` unless
    /// `output_dir` is set, in which case it defaults to the same as
    /// `<output_dir>/<output_subdir>`.
    pub fn output_src_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.output_src_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the source version for Java compilation.
    ///
    /// Corresponds to the `-source` / `--source` flag.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .source_version("11")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn source_version(&mut self, version: impl Into<String>) -> &mut Self {
        self.source_version = Some(version.into());
        self
    }

    /// Set the target version for Java compilation.
    ///
    /// Corresponds to the `-target` / `--target` flag.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .target_version("11")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn target_version(&mut self, version: impl Into<String>) -> &mut Self {
        self.target_version = Some(version.into());
        self
    }

    /// Set the release version for Java compilation.
    ///
    /// This is similar to setting both source and target versions and restricting
    /// the standard library APIs available to those of the specified release.
    ///
    /// Corresponds to the `--release` flag.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .release("11")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// If running on a Java 8 or earlier compiler, this flag is not supported
    /// and will return [`crate::Error::Unsupported`]
    pub fn release(&mut self, version: impl Into<String>) -> &mut Self {
        self.release = Some(version.into());
        self
    }

    /// Set the encoding for Java source files.
    ///
    /// Corresponds to the `-encoding` flag.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .encoding("UTF-8")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn encoding(&mut self, encoding: impl Into<String>) -> &mut Self {
        self.encoding = Some(encoding.into());
        self
    }

    /// Enable debug information in compiled classes.
    ///
    /// Corresponds to the `-g` flag.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .debug(true)
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn debug(&mut self, enable: bool) -> &mut Self {
        self.debug = enable;
        self
    }

    /// Enable or disable warnings.
    ///
    /// Corresponds to the `-nowarn` flag when disabled.
    pub fn warnings(&mut self, enable: bool) -> &mut Self {
        self.warnings = enable;
        self
    }

    /// Treat warnings as errors.
    ///
    /// Corresponds to the `-Werror` flag.
    pub fn werror(&mut self, enable: bool) -> &mut Self {
        self.werror = enable;
        self
    }

    /// Add an arbitrary argument to the javac command.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .arg("-parameters")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn arg(&mut self, arg: impl Into<String>) -> &mut Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Add multiple arbitrary arguments to the javac command.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .args(&["-parameters", "-Xlint:unchecked"])
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn args<Iter>(&mut self, args: Iter) -> &mut Self
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<OsStr>,
    {
        for arg in args {
            if let Some(arg_str) = arg.as_ref().to_str() {
                self.extra_args.push(arg_str.to_string());
            }
        }
        self
    }

    /// Remove an argument from the javac command.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .arg("-parameters")
    ///     .remove_arg("-parameters")
    ///     .compile();
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_arg(&mut self, arg: &str) -> &mut Self {
        self.extra_args.retain(|a| a != arg);
        self
    }

    /// Compile the Java sources and return the paths to the generated `.class` files.
    ///
    /// This method will:
    /// 1. Locate the `javac` compiler (via `JAVA_HOME` or `PATH`)
    /// 2. Collect all source files (from `.file()`, `.files()`, and `.source_dir()`)
    /// 3. Invoke `javac` with all configured options
    /// 4. Return the paths to all generated `.class` files
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The `javac` compiler cannot be found
    /// - No source files were specified
    /// - Compilation fails
    /// - The output directory cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() {
    /// let class_files = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .compile();
    ///
    /// for class_file in class_files {
    ///     println!("Compiled: {}", class_file.display());
    /// }
    /// # }
    /// ```
    pub fn compile(&self) -> Vec<PathBuf> {
        let compiler = self.get_compiler();
        compiler.compile()
    }

    /// Try to compile the Java sources, returning a `Result`.
    ///
    /// This is the non-panicking version of `compile()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `javac` compiler cannot be found
    /// - No source files were specified
    /// - Compilation fails
    /// - The output directory cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let class_files = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .try_compile()?;
    ///
    /// for class_file in class_files {
    ///     println!("Compiled: {}", class_file.display());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_compile(&self) -> Result<Vec<PathBuf>> {
        let compiler = self.try_get_compiler()?;
        compiler.try_compile()
    }

    /// Get a configured `JavaCompiler` that represents the current build configuration.
    ///
    /// This returns a snapshot of the build state that can be used to query the compiler
    /// path or convert to a `Command` for manual execution.
    ///
    /// # Panics
    ///
    /// Panics if the `javac` compiler cannot be found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .get_compiler();
    ///
    /// println!("Using javac at: {}", compiler.path().display());
    /// # }
    /// ```
    pub fn get_compiler(&self) -> JavaCompiler {
        self.try_get_compiler()
            .unwrap_or_else(|err| panic!("failed to find javac compiler: {err}"))
    }

    /// Try to get a configured `JavaCompiler`, returning an error if the compiler cannot be found.
    ///
    /// This is the non-panicking version of `get_compiler()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .try_get_compiler()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_get_compiler(&self) -> Result<JavaCompiler> {
        JavaCompiler::new(self)
    }

    /// Find the `javac` executable.
    ///
    /// First checks if `JAVA_HOME` is set and looks for `javac` in `JAVA_HOME/bin`.
    /// If not found, falls back to searching in `PATH`.
    fn find_javac(&self) -> Result<PathBuf> {
        // Try JAVA_HOME first
        if let Ok(java_home) = env::var("JAVA_HOME") {
            let javac_path = Path::new(&java_home).join("bin").join(if cfg!(windows) {
                "javac.exe"
            } else {
                "javac"
            });

            if javac_path.exists() {
                return Ok(javac_path);
            }
        }

        // Fall back to PATH
        which::which("javac").map_err(|e| {
            Error::CompilerNotFound(format!(
                "Could not find javac: {}. Please set JAVA_HOME or ensure javac is in PATH.",
                e
            ))
        })
    }

    /// Check javac version and emit a warning if < 19.
    ///
    /// Parses output like "javac 25.0.1" or "javac 1.8.0_472".
    /// Returns the major version number.
    fn check_javac_version(&self, javac: &Path) -> Result<u32> {
        let output = Command::new(javac).arg("-version").output()?;

        // Decode as UTF-8 (javac -version typically outputs UTF-8 or ASCII)
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // javac -version outputs to stderr
        let version_output = stderr.as_ref();
        let stdout_output = stdout.as_ref();

        // Check both stderr and stdout
        let actual_output = if !version_output.trim().is_empty() {
            version_output
        } else if !stdout_output.trim().is_empty() {
            stdout_output
        } else {
            return Err(Error::CompilerNotFound(
                "Could not capture javac version output".to_string(),
            ));
        };

        // Parse version number (e.g., "javac 25.0.1" -> 25, "javac 1.8.0_472" -> 8)
        if let Some(version_str) = actual_output.split_whitespace().nth(1) {
            let parts: Vec<&str> = version_str.split('.').collect();

            // Handle Java 8 and earlier: "1.8.0_472" -> major version is 8
            if parts.len() >= 2
                && parts[0] == "1"
                && let Ok(minor) = parts[1].split('_').next().unwrap_or("").parse::<u32>()
            {
                return Ok(minor);
            }

            // Handle Java 9+: "25.0.1" -> major version is 25
            if let Some(major_str) = parts.first()
                && let Ok(major) = major_str.parse::<u32>()
            {
                return Ok(major);
            }
        }

        Err(Error::CompilerNotFound(format!(
            "Could not parse javac version from output: '{}'",
            actual_output.trim()
        )))
    }

    /// Collect all source files from explicit files and source directories.
    fn collect_source_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = self.files.clone();

        // Scan source directories for .java files
        for source_dir in &self.source_dirs {
            Self::scan_directory(source_dir, &mut files)?;
        }

        Ok(files)
    }

    /// Recursively scan a directory for `.java` files.
    fn scan_directory(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Err(Error::InvalidDirectory(dir.display().to_string()));
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::scan_directory(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("java") {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Get the output directory, defaulting to OUT_DIR if not set.
    /// If a subdirectory is specified, it is appended to the base directory.
    fn get_output_dir(&self) -> Result<PathBuf> {
        // If output_dir is set, use it, else default to $OUT_DIR/javac-build/classes/
        let base_dir = if let Some(ref dir) = self.output_dir {
            dir.clone()
        } else {
            let out_dir = env::var("OUT_DIR")
                .map(PathBuf::from)
                .map_err(|_| Error::OutDirNotSet)?;
            out_dir.join("javac-build/classes/")
        };
        Ok(if let Some(ref subdir) = self.output_subdir {
            base_dir.join(subdir)
        } else {
            base_dir
        })
    }
}

/// Represents a configured Java compiler ready to execute.
///
/// This provides access to the resolved compiler path and can be converted to a
/// `Command` for manual execution or used directly to compile sources.
#[derive(Debug, Clone)]
pub struct JavaCompiler {
    path: PathBuf,
    source_files: Vec<PathBuf>,
    source_dirs: Vec<PathBuf>,
    output_dir: PathBuf,
    output_src_dir: PathBuf,
    classpath: Vec<PathBuf>,
    source_version: Option<String>,
    target_version: Option<String>,
    release: Option<String>,
    encoding: Option<String>,
    debug: bool,
    warnings: bool,
    werror: bool,
    extra_args: Vec<String>,
    emit_rerun_if_changed: bool,
    cargo_metadata: bool,
    javac_version: u32,
}

impl JavaCompiler {
    /// Create a new `JavaCompiler` from a `Build` configuration.
    fn new(build: &Build) -> Result<Self> {
        let path = build.find_javac()?;
        let javac_version = build.check_javac_version(&path)?;
        let source_files = build.collect_source_files()?;
        let output_dir = build.get_output_dir()?;
        let output_src_dir = build.get_output_src_dir()?;
        Ok(JavaCompiler {
            path,
            source_files,
            source_dirs: build.source_dirs.clone(),
            output_dir,
            output_src_dir,
            classpath: build.classpath.clone(),
            source_version: build.source_version.clone(),
            target_version: build.target_version.clone(),
            release: build.release.clone(),
            encoding: build.encoding.clone(),
            debug: build.debug,
            warnings: build.warnings,
            werror: build.werror,
            extra_args: build.extra_args.clone(),
            emit_rerun_if_changed: build.emit_rerun_if_changed,
            cargo_metadata: build.cargo_metadata,
            javac_version,
        })
    }

    /// Get the path to the `javac` executable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .get_compiler();
    ///
    /// println!("Using javac at: {}", compiler.path().display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Convert this compiler into a `Command` ready to execute.
    ///
    /// This allows manual execution or further customization of the javac invocation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .get_compiler();
    ///
    /// let mut cmd = compiler.to_command()?;
    /// cmd.env("CUSTOM_VAR", "value");
    /// let output = cmd.output()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_command(&self) -> Result<Command> {
        if self.source_files.is_empty() {
            return Err(Error::NoSourceFiles);
        }

        fs::create_dir_all(&self.output_dir)?;
        fs::create_dir_all(&self.output_src_dir)?;

        let mut cmd = Command::new(&self.path);

        // Always set file.encoding=UTF-8 so @file lists are read as UTF-8 on all platforms
        cmd.arg("-J-Dfile.encoding=UTF-8");

        // JDK 18 workaround for stdout/stderr encoding
        cmd.arg("-J-Dsun.stdout.encoding=UTF-8");
        cmd.arg("-J-Dsun.stderr.encoding=UTF-8");

        // JDK 19+ standard properties
        cmd.arg("-J-Dstdout.encoding=UTF-8");
        cmd.arg("-J-Dstderr.encoding=UTF-8");

        // Add output directory
        cmd.arg("-d").arg(&self.output_dir);
        // Add generated source output directory
        cmd.arg("-s").arg(&self.output_src_dir);

        // Add classpath if specified
        if !self.classpath.is_empty() {
            let classpath = env::join_paths(&self.classpath)
                .map_err(|e| Error::InvalidClasspath(format!("{}", e)))?;
            cmd.arg("-classpath").arg(classpath);
        }

        // Add version flags
        if let Some(ref release) = self.release {
            // --release is only supported in Java 9+
            if self.javac_version <= 8 {
                return Err(Error::Unsupported(format!(
                    "--release flag is not supported in Java {} (requires Java 9+)",
                    self.javac_version
                )));
            }
            cmd.arg("--release").arg(release);
        } else {
            if let Some(ref source) = self.source_version {
                cmd.arg("-source").arg(source);
            }
            if let Some(ref target) = self.target_version {
                cmd.arg("-target").arg(target);
            }
        }

        // Add encoding flag if specified
        if let Some(ref encoding) = self.encoding {
            cmd.arg("-encoding").arg(encoding);
        }

        // Add debug flag
        if self.debug {
            cmd.arg("-g");
        }

        // Always add verbose flag to ensure [wrote ...] lines are emitted
        cmd.arg("-verbose");

        // Add warning flags
        if !self.warnings {
            cmd.arg("-nowarn");
        }
        if self.werror {
            cmd.arg("-Werror");
        }

        // Add extra arguments
        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        // Create @file list
        let file_list = self.create_file_list()?;
        cmd.arg(format!("@{}", file_list.display()));

        Ok(cmd)
    }

    /// Compile the Java sources and return the paths to the generated `.class` files.
    ///
    /// # Panics
    ///
    /// Panics if compilation fails or if the output cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .get_compiler();
    ///
    /// let class_files = compiler.compile();
    /// # }
    /// ```
    pub fn compile(&self) -> Vec<PathBuf> {
        self.try_compile().unwrap_or_else(|err| panic!("{err}"))
    }

    /// Try to compile the Java sources, returning a `Result`.
    ///
    /// This is the non-panicking version of `compile()`.
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails or if the output cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), javac::Error> {
    /// let compiler = javac::Build::new()
    ///     .file("src/Foo.java")
    ///     .get_compiler();
    ///
    /// let class_files = compiler.try_compile()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_compile(&self) -> Result<Vec<PathBuf>> {
        // Only emit cargo metadata if enabled
        if self.cargo_metadata {
            // Always emit rerun-if-env-changed=JAVA_HOME
            println!("cargo:rerun-if-env-changed=JAVA_HOME");
            if self.emit_rerun_if_changed {
                use std::collections::HashSet;
                let mut seen: HashSet<PathBuf> = HashSet::new();
                // Emit for source files (deduped)
                for f in &self.source_files {
                    if seen.insert(f.clone()) {
                        println!("cargo:rerun-if-changed={}", f.display());
                    }
                }
                // Emit for source directories so new files trigger rebuilds
                for d in &self.source_dirs {
                    if seen.insert(d.clone()) {
                        println!("cargo:rerun-if-changed={}", d.display());
                    }
                }
            }
        }

        let mut cmd = self.to_command()?;

        // Execute compilation
        let output = cmd.output()?;

        // Clean up temporary file list
        let file_list = self.output_dir.join("javac_file_list.txt");
        let _ = fs::remove_file(&file_list);

        // Decode stderr/stdout as UTF-8
        let stderr = String::from_utf8(output.stderr)?;
        let stdout = String::from_utf8(output.stdout)?;

        if !output.status.success() {
            return Err(Error::CompilationFailed { stdout, stderr });
        }

        // Parse stderr lines of the form: [wrote path/to/Foo.class]
        // or in Java 8: [wrote RegularFileObject[path/to/Foo.class]]
        let mut seen = std::collections::HashSet::new();
        let mut class_files = Vec::new();
        for line in stderr.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("[wrote ")
                && let Some(end_idx) = rest.rfind(']')
            {
                let mut path_str = &rest[..end_idx];

                // Handle Java 8 format: RegularFileObject[/path/to/File.class]
                if let Some(inner_path) = path_str.strip_prefix("RegularFileObject[") {
                    // Also strip the trailing ] that's part of the RegularFileObject wrapper
                    if let Some(final_path) = inner_path.strip_suffix("]") {
                        path_str = final_path;
                    }
                }

                if path_str.ends_with(".class") && seen.insert(path_str.to_owned()) {
                    class_files.push(PathBuf::from(path_str));
                }
            }
        }

        // Error if no [wrote ...] lines were found
        if class_files.is_empty() {
            return Err(Error::NoClassFilesGenerated { stdout, stderr });
        }

        Ok(class_files)
    }

    /// Create a temporary file containing the list of source files for javac.
    fn create_file_list(&self) -> Result<PathBuf> {
        let file_list_path = self.output_dir.join("javac_file_list.txt");
        let mut file = File::create(&file_list_path)?;

        for source_file in &self.source_files {
            // Use `path::absolute()` to avoid UNC path issues on Windows
            let abs_path = std::path::absolute(source_file).unwrap_or_else(|_| source_file.clone());
            let path_str = abs_path.display().to_string();

            writeln!(file, "{}", path_str)?;
        }

        Ok(file_list_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        let build = Build::new()
            .file("Foo.java")
            .file("Bar.java")
            .source_version("11")
            .target_version("11")
            .debug(true)
            .clone();

        assert_eq!(build.files.len(), 2);
        assert_eq!(build.source_version, Some("11".to_string()));
        assert_eq!(build.target_version, Some("11".to_string()));
        assert!(build.debug);
    }

    #[test]
    fn test_files_method() {
        let files = vec!["Foo.java", "Bar.java", "Baz.java"];
        let build = Build::new().files(files).clone();

        assert_eq!(build.files.len(), 3);
    }

    #[test]
    fn test_only_reported_class_files() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&out_dir)?;

        // Java sources: Foo.java compiled, Bar.java exists but we won't compile it.
        let foo_java = src_dir.join("Foo.java");
        let bar_java = src_dir.join("Bar.java");
        fs::write(&foo_java, "public class Foo { }\n")?;
        fs::write(&bar_java, "public class Bar { }\n")?;

        // Create an unrelated pre-existing class file that should NOT appear in results.
        let unrelated = out_dir.join("Unrelated.class");
        fs::write(&unrelated, b"CAFEBABE")?; // minimal placeholder bytes

        // Build compiling only Foo.java
        let result = Build::new().file(&foo_java).output_dir(&out_dir).compile();

        println!("Reported class files: {:?}", result);

        // Ensure Foo.class was reported.
        let foo_class = out_dir.join("Foo.class");
        assert!(
            result.contains(&foo_class) || result.contains(&PathBuf::from("Foo.class")),
            "Expected Foo.class in reported class files"
        );

        // Ensure Bar.class was NOT reported (since we did not compile Bar.java).
        let bar_class = out_dir.join("Bar.class");
        assert!(
            !result.contains(&bar_class) && !result.contains(&PathBuf::from("Bar.class")),
            "Bar.class should not be reported"
        );

        // Ensure Unrelated.class was NOT reported.
        assert!(
            !result.contains(&unrelated) && !result.contains(&PathBuf::from("Unrelated.class")),
            "Unrelated.class should not be reported"
        );

        Ok(())
    }

    #[test]
    fn test_unicode_filename() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_unicode_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        // Use Unicode characters that are representable in Windows-1252 (common ANSI code page)
        // but would be corrupted if we incorrectly decoded as UTF-8.
        // These characters have different byte sequences in Windows-1252 vs UTF-8:
        // - ë (U+00EB): 0xEB in Windows-1252, 0xC3 0xAB in UTF-8
        // - ñ (U+00F1): 0xF1 in Windows-1252, 0xC3 0xB1 in UTF-8
        // - é (U+00E9): 0xE9 in Windows-1252, 0xC3 0xA9 in UTF-8
        // If we decode Windows-1252 bytes as UTF-8, we'd get invalid UTF-8 or wrong characters.
        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&out_dir)?;

        // Create a Java file with a Unicode filename that's valid in Windows-1252.
        let unicode_java = src_dir.join("Tëstñamé.java");
        fs::write(&unicode_java, "public class Tëstñamé { }\n")?;

        // Build and compile. We need to tell javac the source file encoding is UTF-8
        // since the file content contains UTF-8 encoded Unicode characters.
        let result = Build::new()
            .file(&unicode_java)
            .output_dir(&out_dir)
            .encoding("UTF-8")
            .compile();

        // Verify that the Unicode filename was correctly parsed from stderr.
        // If we used the wrong encoding, the Unicode characters would be garbled.
        assert!(
            !result.is_empty(),
            "Expected at least one class file to be reported"
        );

        // Check that the result contains a path with our Unicode filename.
        // If encoding was wrong, the garbled path won't match.
        let found_unicode = result.iter().any(|p| {
            p.to_str()
                .map(|s| s.contains("Tëstñamé.class"))
                .unwrap_or(false)
        });

        assert!(
            found_unicode,
            "Expected to find Tëstñamé.class in results, but got: {:?}\n\
             This likely means stderr encoding detection is incorrect.",
            result
        );

        // Also verify the class file actually exists on disk.
        let expected_class = out_dir.join("Tëstñamé.class");
        assert!(
            expected_class.exists(),
            "Expected class file to exist at {}",
            expected_class.display()
        );

        Ok(())
    }

    #[test]
    fn test_unicode_filename_beyond_windows1252() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_unicode_complex_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        // Use characters that are NOT representable in Windows-1252 but are valid in Java identifiers:
        // - 你好 (Chinese characters, U+4F60 U+597D) - valid Java identifier characters
        // - Привет (Cyrillic characters) - valid Java identifier characters
        // These characters are only properly representable in UTF-8 (or UTF-16).
        // If javac output is decoded as Windows-1252, these would be corrupted.
        // Note: Java identifiers can contain Unicode letters, digits, underscore, and dollar sign.
        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&out_dir)?;

        // Create a Java file with a filename containing non-Windows-1252 characters
        // Using Chinese and Cyrillic characters which are valid in Java identifiers
        let unicode_java = src_dir.join("Test你好Привет.java");
        fs::write(&unicode_java, "public class Test你好Привет { }\n")?;

        // Build and compile. We need to tell javac the source file encoding is UTF-8
        // since the file content contains UTF-8 encoded Unicode characters.
        let result = Build::new()
            .file(&unicode_java)
            .output_dir(&out_dir)
            .encoding("UTF-8")
            .compile();

        // Verify that the Unicode filename was correctly parsed from stderr.
        assert!(
            !result.is_empty(),
            "Expected at least one class file to be reported"
        );

        // Check that the result contains a path with our Unicode filename.
        // If encoding was wrong, the characters would be corrupted or missing.
        let found_unicode = result.iter().any(|p| {
            p.to_str()
                .map(|s| s.contains("Test你好Привет.class"))
                .unwrap_or(false)
        });

        assert!(
            found_unicode,
            "Expected to find Test你好Привет.class in results, but got: {:?}\n\
             This likely means stderr encoding detection is incorrect for non-Windows-1252 characters.",
            result
        );

        // Also verify the class file actually exists on disk.
        let expected_class = out_dir.join("Test你好Привет.class");
        assert!(
            expected_class.exists(),
            "Expected class file to exist at {}",
            expected_class.display()
        );

        Ok(())
    }

    #[test]
    fn test_utf16_encoding() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_utf16_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&out_dir)?;

        // Create a Java file with Unicode content (emoji and various Unicode characters)
        // and encode it in UTF-16 (with BOM for easy detection by javac)
        let utf16_java = src_dir.join("Utf16Test.java");

        // Java source code with Unicode characters including emoji
        let java_source = "public class Utf16Test {\n    // 你好世界 🌍\n    public static void main(String[] args) {\n        System.out.println(\"Hello UTF-16! 🎉\");\n    }\n}\n";

        // Encode as UTF-16LE with BOM
        let mut utf16_bytes = vec![0xFF, 0xFE]; // UTF-16LE BOM
        for c in java_source.encode_utf16() {
            utf16_bytes.push((c & 0xFF) as u8);
            utf16_bytes.push((c >> 8) as u8);
        }

        fs::write(&utf16_java, utf16_bytes)?;

        // Build and compile, specifying UTF-16 encoding
        let result = Build::new()
            .file(&utf16_java)
            .output_dir(&out_dir)
            .encoding("UTF-16")
            .compile();

        // Verify compilation succeeded and produced the class file
        assert!(
            !result.is_empty(),
            "Expected at least one class file to be reported"
        );

        let expected_class = out_dir.join("Utf16Test.class");
        assert!(
            expected_class.exists(),
            "Expected class file to exist at {}",
            expected_class.display()
        );

        // Verify the class file was reported in results
        let found = result.iter().any(|p| {
            p.to_str()
                .map(|s| s.contains("Utf16Test.class"))
                .unwrap_or(false)
        });

        assert!(
            found,
            "Expected to find Utf16Test.class in results, but got: {:?}",
            result
        );

        Ok(())
    }

    #[test]
    fn test_shift_jis_encoding() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_shiftjis_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&out_dir)?;

        // Create a Java file with Japanese characters encoded in Shift-JIS
        let shiftjis_java = src_dir.join("ShiftJisTest.java");

        // Java source code with Japanese text
        let java_source = "public class ShiftJisTest {\n    // こんにちは世界\n    public static void main(String[] args) {\n        System.out.println(\"こんにちは！\");\n    }\n}\n";

        // Encode as Shift-JIS
        use encoding_rs::SHIFT_JIS;
        let (encoded, _, _) = SHIFT_JIS.encode(java_source);

        fs::write(&shiftjis_java, encoded.as_ref())?;

        // Build and compile, specifying Shift_JIS encoding
        let result = Build::new()
            .file(&shiftjis_java)
            .output_dir(&out_dir)
            .encoding("Shift_JIS")
            .compile();

        // Verify compilation succeeded and produced the class file
        assert!(
            !result.is_empty(),
            "Expected at least one class file to be reported"
        );

        let expected_class = out_dir.join("ShiftJisTest.class");
        assert!(
            expected_class.exists(),
            "Expected class file to exist at {}",
            expected_class.display()
        );

        // Verify the class file was reported in results
        let found = result.iter().any(|p| {
            p.to_str()
                .map(|s| s.contains("ShiftJisTest.class"))
                .unwrap_or(false)
        });

        assert!(
            found,
            "Expected to find ShiftJisTest.class in results, but got: {:?}",
            result
        );

        Ok(())
    }

    #[test]
    fn test_output_subdir() -> Result<()> {
        // Create a temporary workspace.
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_output_subdir_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let base_out = temp_root.join("base");
        let src_dir = temp_root.join("src");
        fs::create_dir_all(&src_dir)?;

        // Create two simple Java files
        let foo_java = src_dir.join("Foo.java");
        let bar_java = src_dir.join("Bar.java");
        fs::write(&foo_java, "public class Foo { }\n")?;
        fs::write(&bar_java, "public class Bar { }\n")?;

        // Compile Foo to base/foo-classes/
        Build::new()
            .file(&foo_java)
            .output_dir(&base_out)
            .output_subdir("foo-classes")
            .compile();

        // Compile Bar to base/bar-classes/
        Build::new()
            .file(&bar_java)
            .output_dir(&base_out)
            .output_subdir("bar-classes")
            .compile();

        // Verify both class files exist in their respective subdirectories
        let foo_class = base_out.join("foo-classes").join("Foo.class");
        let bar_class = base_out.join("bar-classes").join("Bar.class");

        assert!(
            foo_class.exists(),
            "Expected Foo.class at {}",
            foo_class.display()
        );
        assert!(
            bar_class.exists(),
            "Expected Bar.class at {}",
            bar_class.display()
        );

        // Verify they're in separate directories
        assert_ne!(
            foo_class.parent().unwrap(),
            bar_class.parent().unwrap(),
            "Class files should be in separate subdirectories"
        );

        Ok(())
    }

    #[test]
    fn test_args_api() {
        let build = Build::new()
            .file("Foo.java")
            .arg("-Xlint:all")
            .args(["-encoding", "UTF-8"])
            .clone();

        assert_eq!(build.extra_args.len(), 3);
        assert_eq!(build.extra_args[0], "-Xlint:all");
        assert_eq!(build.extra_args[1], "-encoding");
        assert_eq!(build.extra_args[2], "UTF-8");
    }

    #[test]
    fn test_remove_arg() {
        let build = Build::new()
            .file("Foo.java")
            .arg("-g")
            .arg("-Xlint:all")
            .arg("-deprecation")
            .remove_arg("-Xlint:all")
            .clone();

        assert_eq!(build.extra_args.len(), 2);
        assert!(build.extra_args.contains(&"-g".to_string()));
        assert!(build.extra_args.contains(&"-deprecation".to_string()));
        assert!(!build.extra_args.contains(&"-Xlint:all".to_string()));
    }

    #[test]
    fn test_get_compiler() -> Result<()> {
        // Create a temporary workspace
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_get_compiler_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;

        let foo_java = src_dir.join("Foo.java");
        fs::write(&foo_java, "public class Foo { }\n")?;

        // Test get_compiler returns a JavaCompiler
        let compiler = Build::new()
            .file(&foo_java)
            .output_dir(&out_dir)
            .get_compiler();

        // Verify we can access the path
        let path = compiler.path();
        assert!(
            path.to_str().unwrap().contains("javac")
                || path.to_str().unwrap().contains("javac.exe")
        );

        Ok(())
    }

    #[test]
    fn test_java_compiler_to_command() -> Result<()> {
        // Create a temporary workspace
        let temp_root = std::env::temp_dir().join(format!(
            "javac_test_to_command_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let src_dir = temp_root.join("src");
        let out_dir = temp_root.join("classes");
        fs::create_dir_all(&src_dir)?;

        let foo_java = src_dir.join("Foo.java");
        fs::write(&foo_java, "public class Foo { }\n")?;

        let compiler = Build::new()
            .file(&foo_java)
            .output_dir(&out_dir)
            .arg("-g")
            .get_compiler();

        // Convert to Command
        let cmd = compiler.to_command()?;

        // Verify the command has the expected program
        let program = cmd.get_program();
        assert!(
            program.to_str().unwrap().contains("javac")
                || program.to_str().unwrap().contains("javac.exe"),
            "Expected javac in command program, got: {:?}",
            program
        );

        Ok(())
    }

    #[test]
    fn test_try_get_compiler() {
        // With a valid setup, it should return Some
        let mut valid_build = Build::new();
        valid_build.file("Foo.java");
        let result = valid_build.try_get_compiler();

        // If javac is available on the system, this should succeed
        // If not, it will return Err which is also acceptable
        if let Ok(compiler) = result {
            // Verify the compiler has a valid path
            let path = compiler.path();
            assert!(
                path.to_str().unwrap().contains("javac")
                    || path.to_str().unwrap().contains("javac.exe"),
                "Expected javac in compiler path"
            );
        }
    }
}
