package jni.it;

import static org.assertj.core.api.Assertions.assertThat;

import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

@NativeTest
class JniTest {

  @ParameterizedTest
  @ValueSource(ints = {Integer.MIN_VALUE + 1, -1, 0, 1, Integer.MAX_VALUE})
  void absStaticIntCall(int x) {
    assertThat(StaticJniCalls.abs(x)).isEqualTo(Math.abs(x));
  }
}
