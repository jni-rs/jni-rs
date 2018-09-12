package jni.it;

final class LibraryLoader {

  private static final String LIB_NAME = "java_test";

  static void loadLibrary() {
    System.loadLibrary(LIB_NAME);
  }

  private LibraryLoader() {}
}
