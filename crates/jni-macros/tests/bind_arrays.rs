use std::fs;
use std::path::{Path, PathBuf};

use jni::Env;
use jni::objects::{JIntArray, JObjectArray, JString};
use jni::{bind_java_type, jni_str};
use rusty_fork::rusty_fork_test;

mod util;

// Create bindings for TestArrays class
bind_java_type! {
    rust_type = TestArrays,
    java_type = "com.example.TestArrays",
    constructors {
        fn new(),
        fn new_with_arrays(int_array: jint[], string_array: JString[]),
    },
    fields {
        static static_int_array: jint[],
        static static_string_array: JString[],
        static static_int_2d_array {
            sig = jint[][],
            name = "staticInt2DArray",
        },
        int_array: jint[],
        long_array: jlong[],
        string_array: JString[],
        int_2d_array {
            sig = jint[][],
            name = "int2DArray",
        },
        string_2d_array {
            sig = JString[][],
            name = "string2DArray",
        },
    },
    methods {
        static fn sum_array(values: jint[]) -> jint,
        static fn concatenate_arrays(arr1: JString[], arr2: JString[]) -> JString[],
        static fn transpose(matrix: jint[][]) -> jint[][],
        static fn create_int_array(size: jint, value: jint) -> jint[],
        static fn create_string_array(size: jint, prefix: JString) -> JString[],
        fn update_int_array(values: jint[]) -> void,
        fn update_string_array(values: JString[]) -> void,
        fn update_2d_array {
            sig = (values: jint[][]) -> void,
            name = "update2DArray",
        },
        fn get_int_array() -> jint[],
        fn get_string_array() -> JString[],
        fn get_2d_array {
            sig = () -> jint[][],
            name = "get2DArray",
        },
        fn double_values(input: jint[]) -> jint[],
        fn to_upper_case(input: JString[]) -> JString[],
        fn get_int_array_length() -> jint,
        fn get_string_array_length() -> jint,
    }
}

rusty_fork_test! {
#[test]
fn test_static_array_fields() {
    let out_dir = setup_test_output("bind_arrays_static_fields");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Test reading static int array field
        let int_array = TestArrays::static_int_array(env)?;
        let int_values = read_int_array(env, &int_array)?;
        assert_eq!(int_values, vec![1, 2, 3, 4, 5]);

        // Test reading static string array field
        let string_array = TestArrays::static_string_array(env)?;
        let string_values = read_string_array(env, &string_array)?;
        assert_eq!(string_values, vec!["hello", "world"]);

        // Test reading static 2D array field
        let int_2d_array = TestArrays::static_int_2d_array(env)?;
        let first_row = int_2d_array.get_element(env, 0)?;
        let first_row_values = read_int_array(env, &first_row)?;
        assert_eq!(first_row_values, vec![1, 2]);

        Ok(())
    })
    .expect("Static array fields test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_array_fields() {
    let out_dir = setup_test_output("bind_arrays_instance_fields");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        let obj = TestArrays::new(env)?;

        // Test reading instance int array field
        let int_array = obj.int_array(env)?;
        let int_values = read_int_array(env, &int_array)?;
        assert_eq!(int_values, vec![10, 20, 30]);

        // Test reading instance long array field
        let long_array = obj.long_array(env)?;
        let long_values = read_long_array(env, &long_array)?;
        assert_eq!(long_values, vec![100, 200, 300]);

        // Test reading instance string array field
        let string_array = obj.string_array(env)?;
        let string_values = read_string_array(env, &string_array)?;
        assert_eq!(string_values, vec!["foo", "bar", "baz"]);

        Ok(())
    })
    .expect("Instance array fields test failed");
}
}

rusty_fork_test! {
#[test]
fn test_array_field_write() {
    let out_dir = setup_test_output("bind_arrays_field_write");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        let obj = TestArrays::new(env)?;

        // Create and set a new int array
        let new_int_array = env.new_int_array(4)?;
        new_int_array.set_region(env, 0, &[100, 200, 300, 400])?;
        obj.set_int_array(env, &new_int_array)?;

        // Read back and verify
        let read_array = obj.int_array(env)?;
        let values = read_int_array(env, &read_array)?;
        assert_eq!(values, vec![100, 200, 300, 400]);

        Ok(())
    })
    .expect("Array field write test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_method_with_array_param() {
    let out_dir = setup_test_output("bind_arrays_static_method_param");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Test sum_array method
        let int_array = env.new_int_array(5)?;
        int_array.set_region(env, 0, &[10, 20, 30, 40, 50])?;

        let sum = TestArrays::sum_array(env, &int_array)?;
        assert_eq!(sum, 150);

        Ok(())
    })
    .expect("Static method with array param test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_method_returning_array() {
    let out_dir = setup_test_output("bind_arrays_static_method_return");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Test create_int_array method
        let result_array = TestArrays::create_int_array(env, 5, 42)?;
        let values = read_int_array(env, &result_array)?;
        assert_eq!(values, vec![42, 42, 42, 42, 42]);

        Ok(())
    })
    .expect("Static method returning array test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_method_string_arrays() {
    let out_dir = setup_test_output("bind_arrays_static_string_arrays");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Test concatenate_arrays method
        let arr1 = create_string_array(env, &["hello", "world"])?;
        let arr2 = create_string_array(env, &["foo", "bar"])?;

        let result = TestArrays::concatenate_arrays(env, &arr1, &arr2)?;
        let result_values = read_string_array(env, &result)?;
        assert_eq!(result_values, vec!["hello", "world", "foo", "bar"]);

        Ok(())
    })
    .expect("Static method string arrays test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_method_2d_array() {
    let out_dir = setup_test_output("bind_arrays_static_2d");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Create a 2x3 matrix - need to create it with correct type
        let row1 = env.new_int_array(3)?;
        row1.set_region(env, 0, &[1, 2, 3])?;
        let row2 = env.new_int_array(3)?;
        row2.set_region(env, 0, &[4, 5, 6])?;

        let matrix = create_2d_int_array(env, &[&[1, 2, 3], &[4, 5, 6]])?;

        // Transpose it (should become 3x2)
        let transposed = TestArrays::transpose(env, &matrix)?;

        // Verify first row of transposed matrix
        let first_row = transposed.get_element(env, 0)?;
        let first_row_values = read_int_array(env, &first_row)?;
        assert_eq!(first_row_values, vec![1, 4]);

        // Verify second row
        let second_row = transposed.get_element(env, 1)?;
        let second_row_values = read_int_array(env, &second_row)?;
        assert_eq!(second_row_values, vec![2, 5]);

        Ok(())
    })
    .expect("Static method 2D array test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_method_with_array_param() {
    let out_dir = setup_test_output("bind_arrays_instance_method_param");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        let obj = TestArrays::new(env)?;

        // Create and set array using method
        let new_array = env.new_int_array(3)?;
        new_array.set_region(env, 0, &[7, 8, 9])?;
        obj.update_int_array(env, &new_array)?;

        // Verify using getter method
        let result = obj.get_int_array(env)?;
        let values = read_int_array(env, &result)?;
        assert_eq!(values, vec![7, 8, 9]);

        Ok(())
    })
    .expect("Instance method with array param test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_method_array_processing() {
    let out_dir = setup_test_output("bind_arrays_instance_processing");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        let obj = TestArrays::new(env)?;

        // Test double_values method
        let input = env.new_int_array(4)?;
        input.set_region(env, 0, &[1, 2, 3, 4])?;

        let result = obj.double_values(env, &input)?;
        let values = read_int_array(env, &result)?;
        assert_eq!(values, vec![2, 4, 6, 8]);

        Ok(())
    })
    .expect("Instance method array processing test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_method_string_array_processing() {
    let out_dir = setup_test_output("bind_arrays_string_processing");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        let obj = TestArrays::new(env)?;

        // Test to_upper_case method
        let input = create_string_array(env, &["hello", "world"])?;

        let result = obj.to_upper_case(env, &input)?;
        let values = read_string_array(env, &result)?;
        assert_eq!(values, vec!["HELLO", "WORLD"]);

        Ok(())
    })
    .expect("Instance method string array processing test failed");
}
}

rusty_fork_test! {
#[test]
fn test_constructor_with_arrays() {
    let out_dir = setup_test_output("bind_arrays_constructor");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Create arrays
        let int_array = env.new_int_array(3)?;
        int_array.set_region(env, 0, &[99, 88, 77])?;

        let string_array = create_string_array(env, &["test", "array"])?;

        // Create object with constructor
        let obj = TestArrays::new_with_arrays(env, &int_array, &string_array)?;

        // Verify arrays were set correctly
        let result_int = obj.get_int_array(env)?;
        let int_values = read_int_array(env, &result_int)?;
        assert_eq!(int_values, vec![99, 88, 77]);

        let result_string = obj.get_string_array(env)?;
        let string_values = read_string_array(env, &result_string)?;
        assert_eq!(string_values, vec!["test", "array"]);

        Ok(())
    })
    .expect("Constructor with arrays test failed");
}
}

rusty_fork_test! {
#[test]
fn test_create_string_array_static_method() {
    let out_dir = setup_test_output("bind_arrays_create_string_array");

    javac::Build::new()
        .file("tests/java/com/example/TestArrays.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_arrays_class(env, &out_dir)?;

        // Test create_string_array method
        let prefix = JString::from_str(env, "item_")?;
        let result = TestArrays::create_string_array(env, 3, &prefix)?;
        let values = read_string_array(env, &result)?;
        assert_eq!(values, vec!["item_0", "item_1", "item_2"]);

        Ok(())
    })
    .expect("Create string array static method test failed");
}
}

// Helper functions

fn load_test_arrays_class(env: &mut Env, out_dir: &Path) -> jni::errors::Result<()> {
    let class_path = out_dir.join("com/example/TestArrays.class");
    assert!(class_path.exists(), "TestArrays.class not found");

    let class_bytes = fs::read(&class_path).expect("Failed to read TestArrays.class");

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    env.define_class(
        Some(jni_str!("com/example/TestArrays")),
        &class_loader,
        &class_bytes,
    )
    .expect("Failed to define TestArrays class");

    Ok(())
}

fn setup_test_output(test_name: &str) -> PathBuf {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jni_macros_tests")
        .join(test_name);

    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}

fn read_int_array(env: &mut Env, array: &JIntArray) -> jni::errors::Result<Vec<i32>> {
    let len = array.len(env)?;
    let mut buffer = vec![0i32; len];
    array.get_region(env, 0, &mut buffer)?;
    Ok(buffer)
}

fn read_long_array(
    env: &mut Env,
    array: &jni::objects::JLongArray,
) -> jni::errors::Result<Vec<i64>> {
    let len = array.len(env)?;
    let mut buffer = vec![0i64; len];
    array.get_region(env, 0, &mut buffer)?;
    Ok(buffer)
}

fn read_string_array(
    env: &mut Env,
    array: &JObjectArray<JString>,
) -> jni::errors::Result<Vec<String>> {
    let len = array.len(env)?;
    let mut result = Vec::new();

    for i in 0..len {
        let jstring = array.get_element(env, i)?;
        let rust_string = jstring.to_string();
        result.push(rust_string);
    }

    Ok(result)
}

fn create_string_array<'local>(
    env: &mut Env<'local>,
    strings: &[&str],
) -> jni::errors::Result<JObjectArray<'local, JString<'local>>> {
    let array = JObjectArray::<JString>::new(env, strings.len(), JString::null())?;

    for (i, &s) in strings.iter().enumerate() {
        let jstring = JString::from_str(env, s)?;
        array.set_element(env, i, &jstring)?;
    }

    Ok(array)
}

fn create_2d_int_array<'local>(
    env: &mut Env<'local>,
    matrix: &[&[i32]],
) -> jni::errors::Result<JObjectArray<'local, JIntArray<'local>>> {
    let array = JObjectArray::<JIntArray>::new(env, matrix.len(), JIntArray::null())?;

    for (i, row) in matrix.iter().enumerate() {
        let row_array = JIntArray::new(env, row.len())?;
        row_array.set_region(env, 0, row)?;
        array.set_element(env, i, &row_array)?;
    }

    Ok(array)
}
