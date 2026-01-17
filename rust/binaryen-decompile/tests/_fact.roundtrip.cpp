// Decompiled from WebAssembly

int32_t func_0(int32_t len) {
  int32_t v = 0;
  int32_t idx = 0;
  v = 1;
  {
    {
      if ((len < 1)) { goto label$0; };
      v = 1;
      idx = 1;
      loop label$1       if (((idx = (idx + (idx < len))) <= len)) { /* break label$1 with value */ break_to_label$0((v = (idx * v))) if (idx >= len); goto label$1; };
    }
  }
label$0: ;
  return v;
}
