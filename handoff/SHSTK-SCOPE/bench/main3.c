/* SHSTK-SCOPE: empty main with three libraries. */
int lib1_fn(int);
int lib2_fn(int);
int lib3_fn(int);
int main(int argc, char **argv)
{
	(void)argv;
	if (argc > 99)
		return lib1_fn(argc) + lib2_fn(argc) + lib3_fn(argc);
	return 0;
}
