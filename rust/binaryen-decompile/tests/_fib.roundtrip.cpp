// Decompiled from WebAssembly

int32_t func_0(int32_t idx0) {
  int32_t idx1 = 0;
  int32_t flag = 0;
  int32_t v = 0;
  {
    {
      if ((idx0 >= 2)) { goto label$0; };
      return (idx0 + 0);
    }
  }
label$0: ;
  idx1 = 0;
  do-while label$1   {
    idx1 = (func_0((idx0 + -1)) + idx1);
    if (flag) { /* break label$1 with value */ (idx0 = (v = (idx0 + -2))); goto label$1; };
  }
  return (v + idx1);
}
