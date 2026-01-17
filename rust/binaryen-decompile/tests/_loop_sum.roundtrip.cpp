// Decompiled from WebAssembly

int32_t func_0(int32_t idx) {
  int32_t v = 0;
  {
    {
      if ((idx >= 1)) { goto label$0; };
      return 0;
    }
  }
label$0: ;
  return ((v = (idx + -1)) + (i32)(((i64)(u32)v * (i64)(u32)(idx + -2)) >>> 1));
}
