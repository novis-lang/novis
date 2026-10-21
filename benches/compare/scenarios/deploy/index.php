<?php
// Prints the version of each part. The driver writes the parts and later replaces them.
// `DEPLOY_PARTS` is the number of parts, 200 unless the row sets it.
$parts = (int) (getenv('DEPLOY_PARTS') ?: 200);
$digits = strlen((string) ($parts - 1));
for ($i = 0; $i < $parts; $i++) {
    require sprintf("%s/parts/p%0{$digits}d.php", __DIR__, $i);
}
echo "\n";
