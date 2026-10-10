<?php
// Prints the version of each of the 200 parts. The driver writes the parts and later replaces them.
for ($i = 0; $i < 200; $i++) {
    require sprintf('parts/p%03d.php', $i);
}
echo "\n";
