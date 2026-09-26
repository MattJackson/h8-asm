# Encoding and API stability

The shared codec retains register classes, addressing forms, operand widths
and explicit noncanonical choices. Decoding and re-encoding an accepted
instruction must preserve its consumed bytes exactly. Rendering follows
Renesas syntax; a GNU compatibility adaptation belongs in the oracle, not
in public output.

Encoding::Standard selects the canonical form for the supplied typed operands.
Widths are part of those operands: a 16-bit and a 32-bit displacement remain
different requests even when their values are equal. Encoding::SxAlternative
identifies a noncanonical row in the bundled H8SX Rev.4 table. Preserve those
row identities when extending the tables; do not reorder existing identities
silently. It contains no cached instruction bytes. The documented H8S
DisplacementStoreAlias similarly retains the permitted alternate opcode bit.

Invalid register numbers, mismatched widths, unsupported target/mode pairs,
reserved fields and incompatible encoding choices must be refused. Callers
should construct instructions through the documented vocabulary, not depend
on private table layout or Debug output.

Correcting an accepted but invalid encoding can narrow the accepted language.
Such a correction needs a manual citation, regression tests and a changelog
entry. A different canonical encoding or public API shape requires review
against the applicable semantic-versioning policy. During 0.x, incompatible
public changes require a minor-version increase. Do not hide behavior changes
behind unchanged coverage percentages.

The original narrow H8/300 and H8SX APIs remain supported alongside the shared
codec. Their deliberately limited acceptance is documented separately.
