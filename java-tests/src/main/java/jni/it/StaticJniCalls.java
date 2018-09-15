package jni.it;

public class StaticJniCalls {

  static {
    LibraryLoader.loadLibrary();
  }

  /** A native abs implementation. */
	public native static int abs(int a);
}
