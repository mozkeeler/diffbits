diffbits
-----
`diffbits` is an experimental library focused on providing and applying patches between sequences of
bytes that differ in relatively few bit positions. For example, this may be effective when used on
two related bloom filters. The diff output should be highly compressible in these situations.


limitations
-----
Currently, `diffbits` cannot handle inputs larger than 536870911 bytes (4GB / 8 - 1). Additionally,
if given two inputs that differ more substantially than at a fraction of bit indices, `diffbits` is
unlikely to perform well.



complexity
-----
`diffbits` runs in linear time with respect to the lengths of its inputs.


format
-----
Given a `left` sequence of bytes and a `right` sequence of bytes (`before`/`after`,
`current`/`next`, etc.), `diffbits` expresses the difference between these inputs as a series of
4-byte big-endian integers (i.e. the bytes that appear earlier are more significant than the bytes
that appear later in each set of 4 bytes). The first integer is the byte length of the `right`
sequence of bytes. The second integer is the bit index of the first bit that differs in the inputs.
Every integer thereafter is the difference from the previous different bit index to the next
different bit index. Each 8-bit byte is treated as big-endian - bits that appear "earlier" in the
byte starting from the left have a higher index than bits that appear "later" (or farther right).

For example, consider the two byte sequences [0xff, 0xfa] and [0xff, 0xf8, 0x03]. The length of the
`right` is 3, or [0x00, 0x00, 0x00, 0x03] encoded as 4 big-endian bytes. The first differing bit is
at index 9. The next differing bit is 7 bits after that bit. The final differing bit is 1 bit after
that. The final output is [0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x07,
0x00, 0x00, 0x00, 0x01].
