import com.prevoty.commons.content.ProtectResult;

class HelloWorld {
    private native Object nativeProtect(String input);
    public static void main(String[] args) {
        System.out.println(new HelloWorld().nativeProtect("Hello, World!"));
    }
    static {
        System.loadLibrary("java_ffi");
    }
}
