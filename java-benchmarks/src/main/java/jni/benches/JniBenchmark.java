package jni.benches;

import java.util.concurrent.TimeUnit;
import jni.it.StaticJniCalls;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.CompilerControl;
import org.openjdk.jmh.annotations.Measurement;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.annotations.Warmup;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@Warmup(iterations = 10, time = 1)
@Measurement(iterations = 10, time = 1)
@State(Scope.Benchmark)
public class JniBenchmark {

  private int x = 5;

  @Benchmark
  public int staticIntCallBaseline() {
    return staticIntCallNonInlined(x);
  }

  @CompilerControl(CompilerControl.Mode.DONT_INLINE)
  private static int staticIntCallNonInlined(int x) {
    return Math.abs(x);
  }

  @Benchmark
  public int staticIntCallJni() {
    return StaticJniCalls.abs(x);
  }
}
