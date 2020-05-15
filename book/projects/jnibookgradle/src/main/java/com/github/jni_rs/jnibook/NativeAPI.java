package com.github.jni_rs.jnibook;

/**
 * The NativeAPI class, which houses all entrypoints to the shared library.
 */
class NativeAPI {

    // Stores any errors that were encountered at library load time
    private static final Throwable INIT_ERROR;

    // The static block below loads the jnibookrs library. It will be
    // executed the first time the NativeAPI is used. Later, it will contain
    // other initialization logic.
    static {
        Throwable error = null;
        try {
            System.loadLibrary("jnibookrs");
        } catch (Throwable t) {
            error = t;
        }
        INIT_ERROR = error;
    }

    private NativeAPI() {
        // Not instantiable
    }

    public static void verifyLink() {
        checkAvailability();
        verify_link();
    }

    static native int verify_link();

    /**
     * Checks whether the library was loaded successfully before calling into a
     * given function, for cleaner exception messages.
     */
    private static void checkAvailability() {
        if (INIT_ERROR != null) {
            throw new RuntimeException(INIT_ERROR);
        }
    }
}

