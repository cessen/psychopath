#!/usr/bin/env python

if __name__ == "__main__":
    text = "static TRAVERSAL_TABLE: [[u8; 48]; 8] = [\n"

    for raydir in range(0, 8):
        ray = [raydir & 1, (raydir >> 1) & 1, (raydir >> 2) & 1]
        
        text += "    ["
        for splits in [[s1, s2, s3] for s3 in range(0,3) for s2 in range(0,3) for s1 in range(0,3)]:
            perm = [0, 1, 2, 3]
            if ray[splits[1]] == 1:
                perm = [perm[1], perm[0]] + perm[2:4]
            if ray[splits[2]] == 1:
                perm = perm[0:2] + [perm[3], perm[2]]
            if ray[splits[0]] == 1:
                perm = perm[2:4] + perm[0:2]
            perm = perm[0] + (perm[1] << 2) + (perm[2] << 4) + (perm[3] << 6)
            text += "%d, " % perm
        text = text[:-1]

        text += "\n     "
        for splits in [[s1, s2] for s2 in range(0,3) for s1 in range(0,3)]:
            perm = [0, 1, 2]
            if ray[splits[1]] == 1:
                perm = [perm[1], perm[0], perm[2]]
            if ray[splits[0]] == 1:
                perm = [perm[2], perm[0], perm[1]]
            perm = perm[0] + (perm[1] << 2) + (perm[2] << 4)
            text += "%d, " % perm
        text = text[:-1]

        text += "\n     "
        for splits in [[s1, s2] for s2 in range(0,3) for s1 in range(0,3)]:
            perm = [0, 1, 2]
            if ray[splits[1]] == 1:
                perm = [perm[0], perm[2], perm[1]]
            if ray[splits[0]] == 1:
                perm = [perm[1], perm[2], perm[0]]
            perm = perm[0] + (perm[1] << 2) + (perm[2] << 4)
            text += "%d, " % perm
        text = text[:-1]
        
        text += "\n     "
        for split in [s1 for s1 in range(0,3)]:
            perm = [0, 1]
            if ray[split] == 1:
                perm = [perm[1], perm[0]]
            perm = perm[0] + (perm[1] << 2)
            text += "%d, " % perm
        text = text[:-1]
        
        text = text[:-1] + "],\n"
    
    text += "];\n"

    print text

