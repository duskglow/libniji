@@owns-macro iseqsig
@@if iseqsig
/* C2X 7.12.17.7: the relational operators raise "invalid" for a quiet NaN too, as the signaling comparison needs. */
# define iseqsig(x, y) (((x) <= (y)) && ((x) >= (y)))
#endif
