//! Zobrist hashing implementation for chess positions
//!
//! Implements position hashing via the Hashable trait:
//! - Hash calculation based on piece placement and position state
//! - Safety feature combines move count, castling rights, and en-passant for
//!   detecting position changes
//! Hash updates are performed in types.rs as pieces/game state changes
use crate::position::Piece;
use crate::prelude::*;

use crate::tt::Hashable;

impl Hashable<usize> for Position {
    fn hash(x: &Self) -> usize {
        x.pos.zobrist()
    }

    fn safety_feature(x: &Self) -> usize {
        x.pos.zobrist()
            ^ (x.castles.hash() * 4654987)
            ^ (x.half_move_count as usize * 98798462468384)
            ^ x.en_passant.to_bb64() as usize
            ^ (x.pos.black.occupied().to_bb64() as usize).wrapping_mul(6541653246798795667)
            ^ (x.pos.white.occupied().to_bb64() as usize).wrapping_mul(9897995300789921388)
    }
}

type ZobristSeed = [[[usize; Player::COUNT]; Piece::COUNT]; Square::COUNT];

pub fn zobrist_hash_bitboard(bb: Bitboard<GenericBB>, pc: Piece, pl: Player) -> usize {
    let mut hash = 0;
    for sq in bb {
        hash ^= ZOBRIST_SEED[sq.to_index() as usize][pc as usize][pl as usize];
    }
    hash
}

pub fn zobrist_hash_square(bb: Bitboard<Square>, pc: Piece, pl: Player) -> usize {
    ZOBRIST_SEED[bb.to_index() as usize][pc as usize][pl as usize]
}

pub fn random_zobrist_seed() -> ZobristSeed {
    let mut z: ZobristSeed = [[[0; Player::COUNT]; Piece::COUNT]; Square::COUNT];
    for i in 0..Square::COUNT {
        for j in 0..Piece::COUNT {
            for k in 0..Player::COUNT {
                z[i][j][k] = std::random::random();
            }
        }
    }
    z
}

pub const ZOBRIST_SEED: ZobristSeed = [
    [
        [17544820912686652937, 12214652826354034474],
        [1892884916427657878, 6505469460538584664],
        [1206720392038753920, 14618855474931515917],
        [4963298248176422982, 14999650231328317088],
        [3190819924565338108, 4368460366903812993],
        [12641035513429281459, 4567311369805783220],
    ],
    [
        [677778136235445375, 12463557472299860834],
        [17001547203730855436, 412192187093110736],
        [1114190944402607030, 8628873833670698987],
        [6534991620319270811, 4524394149417927967],
        [2004757922721971810, 8726618481909509171],
        [15037965053993411447, 2968316085075663447],
    ],
    [
        [1345087501876752337, 8304244592599192313],
        [17915393523439265262, 16011683753978015859],
        [14460948710388751681, 4890359021409762397],
        [3979320027867311413, 215865106250588525],
        [17206178169497755066, 843044126848994294],
        [12698598206606689158, 18248144066388435249],
    ],
    [
        [10552857938743592619, 16437181164062450222],
        [9044079565011625158, 3058409812929857127],
        [14572530270435711395, 5913889686435857506],
        [6535514222634593865, 9939572193310595461],
        [6538722598687604370, 12199226146222947098],
        [835936444577829179, 14976851046085181997],
    ],
    [
        [9589917090123407612, 6215190242770239038],
        [11178337472846225221, 6048724836135992737],
        [1436265403960422336, 10256756512987140287],
        [6895451870061371350, 4468360650510052183],
        [2912621240550409378, 10472036313359002252],
        [7898941832486571771, 7373183581787331839],
    ],
    [
        [11637292043547632601, 809223948484871309],
        [8591921490644608948, 18423684010725700240],
        [11402764206589356970, 6791060037699360400],
        [17742813989899712700, 7955483521235908718],
        [9365460318036389791, 11767033583961044163],
        [13104670129971575970, 1998113794760993590],
    ],
    [
        [6575414959161793448, 17800950910524707085],
        [15484198590649650276, 871944658661953902],
        [6636442673720764111, 15337152507348386766],
        [3274056213418907221, 7190285149981914426],
        [2683580173743530432, 10724810902367028426],
        [9228377922109449413, 16685236331653431663],
    ],
    [
        [16989164849674572419, 7842672981941188808],
        [10752237743402413046, 3644868431511471906],
        [5803929637140763884, 9717527759766051123],
        [4225315092395590577, 12433619789577573936],
        [11110679076751935886, 15232111289150281211],
        [1290839285726030315, 506527220345746250],
    ],
    [
        [8269357322338806647, 3152478234590449389],
        [2443741124729817124, 8864031649079894787],
        [6999786246554530741, 3169052906058944901],
        [14762279390534348546, 7814671285096958661],
        [5920695375280206895, 7312458063087812086],
        [259533128081334095, 12369253219879059762],
    ],
    [
        [1375222319433095634, 5928985656423720637],
        [15444891928062477868, 4314659876809003056],
        [3348934679167366061, 12837712505408581162],
        [419289176255946122, 9053298739845003030],
        [5865556661765835601, 12842078300199951478],
        [16259894447192275163, 12370322245758138364],
    ],
    [
        [8013507583255801353, 10144733342846408570],
        [3731346113918057064, 9103744204353661720],
        [16076764221364744978, 594442858696188544],
        [17318123520438593639, 13699535727502526891],
        [9980609041111251526, 5421379248650161729],
        [6008426670348066717, 4810109830371880548],
    ],
    [
        [1189327425392922414, 12650197244704265506],
        [15133697212057439489, 6869686390323622134],
        [8916984058974593698, 5183101614809992478],
        [11379215746514657386, 12156299758032052255],
        [9298298613185438836, 3959679613856303084],
        [10146409934241860857, 13301514547853570925],
    ],
    [
        [705427365090153735, 8010599753304990195],
        [14768552410830560503, 3071906763545143728],
        [18229570364362173672, 3309531543468708539],
        [4610088657747353374, 6527174178812382959],
        [1175308071777980112, 18371052654884203846],
        [7447148787087091662, 529251139705927926],
    ],
    [
        [1585663361991223885, 530073897773843833],
        [74061364109860888, 8519527428478227711],
        [8827942348463888339, 2827289243599009528],
        [3860314579812710249, 15716389819086304245],
        [13777229124371409476, 18193914635130958644],
        [6169856755344802566, 7439050942878503441],
    ],
    [
        [12330517734308176063, 16476501935592414340],
        [9775168905788026102, 12341844189032615403],
        [14079787589642323102, 1121500988027264576],
        [5140855265976478701, 3332328556596005026],
        [6394740617119449392, 16100483077246509912],
        [3652841913894353674, 7638856146657177977],
    ],
    [
        [13011562389655735302, 17531671188686749904],
        [11542210573023621393, 5774310702196810887],
        [6821353690902508286, 8708364345953468738],
        [14249946046325960974, 4940269978507754574],
        [2638798972825083374, 8383712929991972456],
        [10068469963470631045, 12188565933874467649],
    ],
    [
        [6374766970222259421, 15297249472778598115],
        [9152496075775120510, 2781074635479752414],
        [12185368315153765948, 3946842503639863408],
        [3243622063782761015, 15090384158607907945],
        [13313134877997413905, 3256246490126006441],
        [12435738304325602569, 6541693717904491324],
    ],
    [
        [14944533720510286180, 8751468923490163407],
        [17368397396918274634, 12109851219930504744],
        [6426577668989439521, 5030163504842975933],
        [14818850009070348227, 2352077261778975775],
        [3284006142255378751, 13198427422145596029],
        [10903926086602220377, 18065578627619121408],
    ],
    [
        [17544666803656896811, 231664563845875164],
        [15628694987484844167, 13449341729303628048],
        [17265291444248247192, 14236710760608923128],
        [15776356046364913319, 2860261319219018059],
        [12322311036945271725, 7858901080718089612],
        [16471867202785274156, 2084883973029972784],
    ],
    [
        [5884446308386415787, 12937067785984374730],
        [9851810016172045735, 3366095396119950684],
        [10670444533940126103, 14870245417249659565],
        [16037801826137716055, 11372040211760750897],
        [7653664305459065915, 691422645899133178],
        [14696472197418615346, 9093942906337932649],
    ],
    [
        [9174187114555618037, 17389715652685563317],
        [11099599563914430444, 4148205439030306],
        [10279659948711472744, 9393979080622491392],
        [12694273989206519301, 6855355500701222506],
        [895176235487822594, 13724965001604256898],
        [13993387842112849298, 3637001821641533659],
    ],
    [
        [18196488079135870255, 10879597008607886337],
        [16070549562944514835, 5563662841831750246],
        [3932274414733058433, 12141801154816438999],
        [10257054133187979164, 6177148927330081490],
        [14100464768254927792, 17485575719660297006],
        [1463639157146173166, 11952292726743908800],
    ],
    [
        [2793187558745099963, 15918956545768047725],
        [13545401558187582321, 13757337137963610374],
        [4290576725945614368, 1761611360466821284],
        [13347284629113734890, 11962576848340070144],
        [8492694558553494789, 12173533808839790559],
        [8661330659270343776, 9598233950003123784],
    ],
    [
        [11599733042685951197, 10360280596639629574],
        [14720977375833258308, 8699494196203406839],
        [6620118158534715795, 3161620871568343893],
        [1173099817840894466, 18012822360282341080],
        [12407660080928204792, 4220534588044027],
        [10340079047837689711, 15576969601975058313],
    ],
    [
        [14893488657875213920, 7779730045497765448],
        [6521941702646209041, 15524784044925775102],
        [17614071348355976988, 12921523328904271428],
        [7756183711766408429, 15158112215818309185],
        [253410287186433372, 14362387276740178593],
        [1557759887491175595, 14419953629807553873],
    ],
    [
        [17704069147755636579, 10702589920737129960],
        [6375910383220116611, 152785563015623054],
        [9675808706901593109, 8677436770191677829],
        [158552456349168353, 7447776404924511062],
        [6557092658060934216, 485946466341150838],
        [16845908870375744988, 17142930218578805762],
    ],
    [
        [7554391650482478369, 16018960912089575265],
        [11308565880335861337, 8874063541773291309],
        [13375753601110706895, 12257415512740801031],
        [10686973562303667714, 7079517057893021944],
        [8331600383069451234, 13281063172351273387],
        [8930925373128062903, 15710997703395550981],
    ],
    [
        [13240599340632670825, 6529032991935498066],
        [18080352845273323117, 13735753216723871181],
        [2525943731212787303, 18320125010062689882],
        [11908404517630369980, 10883361376767393921],
        [1462033512152722913, 12526835567155910799],
        [11795127213706007005, 5897915410763174641],
    ],
    [
        [9343481246875495115, 4471474886416403091],
        [7970807162116306127, 5725117308237676988],
        [11293549822632701178, 2939827864534493414],
        [17132505027941835965, 13150575831280454053],
        [483962065470175879, 1094241480373002516],
        [3187970897166669349, 4599314649489710618],
    ],
    [
        [1884836218057292715, 8920716188497390047],
        [9031363652543285839, 15228825028329394572],
        [14751594018746690734, 12375312715510868033],
        [13652001054866426905, 235314331565119557],
        [2171214834121461501, 12033792358344910748],
        [3719920649531427017, 7419727994050154105],
    ],
    [
        [8890894872902721881, 11052746207704536713],
        [4200880622720702926, 4129594890767638742],
        [2599490196883314782, 15289300594868703199],
        [1208723760488382839, 17550476608564870831],
        [10588458646915093536, 2154789807202204990],
        [10797635659350940649, 7300684984736237913],
    ],
    [
        [2085905918264799666, 7263513181868694466],
        [3489500186165630805, 12092983913286914154],
        [11001756194045949785, 1694237791502077869],
        [6713794980051713677, 9755294510276043046],
        [14926563952240587109, 12460643750853582323],
        [6836797860073011270, 2030814363642730700],
    ],
    [
        [9917954385264877536, 15264901106415980651],
        [5615136684047829181, 7782412825293713622],
        [15298366960782105309, 1709115156049637601],
        [2741070172281066109, 4119229968060224756],
        [1528439693884640211, 3738183285452529909],
        [15103267836699423980, 7723055094088309262],
    ],
    [
        [14468091197548553435, 5345177574070208112],
        [3412098955513263578, 13721070666514928987],
        [6522668076592892513, 1019168661672346098],
        [14442806606823511606, 17068371625324538119],
        [3763184628151931805, 12820181299849192403],
        [17658263475704452138, 5774911890624442796],
    ],
    [
        [4606191079207052457, 8994274979985443536],
        [12535656924082635557, 10726558385178952902],
        [5325057882736054016, 9190266597097420067],
        [16960395772921084051, 16445236367125410829],
        [2334395919013063675, 15536467483348521368],
        [4246993808475510524, 12541671912674375464],
    ],
    [
        [16120446482098704774, 8028142387478967988],
        [12499846684218190666, 13398795575445906484],
        [2662770912262011072, 15155250998272323553],
        [7405931955326386, 11009381133698508352],
        [4001266504501428051, 8486719781456217415],
        [26516400597265966, 11864652484606779434],
    ],
    [
        [16598884690170129041, 10960345347542930616],
        [11857123607822807285, 13988287971893236377],
        [13524276767769928749, 13767575041996884906],
        [18436311140262094742, 11047072766706737399],
        [10015329763519132417, 15299209147936626180],
        [5043032577410150540, 6338921152597538664],
    ],
    [
        [4029897018907292951, 11351041160733819275],
        [10738101992646619540, 14571391767907210856],
        [1462250031216925068, 7466631561270808497],
        [9537812756351015894, 11116845149246477523],
        [1801919016733286703, 6694233409758261890],
        [13754689988310261808, 10489244354961667489],
    ],
    [
        [13785778759416776497, 8902364913371056727],
        [3398786334240109214, 3993893207801987013],
        [3325263431805747516, 3632094909542086229],
        [2799362514537976485, 13282461595314263482],
        [1521447690148649935, 6636390212465646955],
        [17694784850922599120, 14818890090741561706],
    ],
    [
        [14108924535880909859, 3825940623979522279],
        [7379574365874757169, 11252737911877084971],
        [6494324823345609844, 10002273248090420682],
        [13824694630815544187, 6682785372446732243],
        [13911503446802930651, 7330332434953409050],
        [1853648975428583637, 17769947862810653665],
    ],
    [
        [6953344301001816241, 13148705514405026845],
        [17899467260745057925, 4592828915308096037],
        [13378406392081603237, 3921172637468261826],
        [17875174037285179479, 5928094787817617904],
        [13270548010563561571, 929227020934658726],
        [5525386318310502829, 6317731196066529960],
    ],
    [
        [17660320854529418779, 17209824701673351029],
        [13661168934988780860, 5832849334112033096],
        [8820509238508544600, 668384483753050057],
        [983914414324129775, 1373424036817216690],
        [11720240663130787491, 14730275610445507461],
        [17480872415195036062, 2449420287797893419],
    ],
    [
        [10464065542213473642, 3279180513018160631],
        [13215865313318819092, 16200663725336029840],
        [3728988802804441353, 17570766480974428920],
        [11945405836985645758, 10056190187177947632],
        [16107919524286845857, 14733891310284989120],
        [2334848702042776740, 5879134784889651255],
    ],
    [
        [856314030844171592, 5093517661300155058],
        [1011527471024510652, 14213631447592651368],
        [9096467146427775405, 185123693954906888],
        [14135393829356235261, 8850372185096639705],
        [125481564303890222, 11405443317985808970],
        [17448074370192257826, 12037450069775798227],
    ],
    [
        [17395665637405939853, 8042046165879808170],
        [15399900519047650826, 17670558925333565886],
        [6182635389116101959, 13065876682895829525],
        [16567226038355445875, 3372147376976982992],
        [2609035355342803815, 1017781304803692882],
        [8883341861739786527, 12917351409539031401],
    ],
    [
        [17323413265497481752, 18358975585973178560],
        [3607024378413187948, 4213373208645475826],
        [5647677655141713440, 12524542016505190827],
        [18050193144625826525, 17370142684107919916],
        [1307706593811855554, 9697352962604559559],
        [3026270698836933300, 2640693750620579296],
    ],
    [
        [8591043471367791842, 2878467335532680275],
        [6595998003285739730, 18044913892149662986],
        [12497189607023235377, 2830700167956483584],
        [11554942475533385404, 1979571015941623142],
        [3784108861594412967, 12756742024656923146],
        [11099623936956594321, 15583197661692078462],
    ],
    [
        [2957114719634156977, 9580507935316559824],
        [6411551868060586744, 13260513238676286191],
        [12274880588768723829, 10364990966004262215],
        [7008555027428618914, 5452087009178486088],
        [16665427751918546916, 705064716118049132],
        [11689367270720805054, 4153677859401867080],
    ],
    [
        [8620907757339603932, 9659513273438161503],
        [6529254642433732333, 7451695485787313851],
        [6906687399232217995, 14630640950232231927],
        [7852739833945125486, 11281018663219244315],
        [4153097041692616148, 17400426649003991642],
        [6335511671232636471, 11844369817496904339],
    ],
    [
        [4333510947543512587, 6070404165674893295],
        [4491800785614449332, 10408405982538890098],
        [12776948913104674959, 5306282333370974473],
        [9757721993076375147, 3501605583848519725],
        [18172395367323503990, 11814203794419576612],
        [13163637741234591792, 5585993108778547122],
    ],
    [
        [2095163638432096855, 5772071249503753885],
        [309304127058717461, 5089940685601058458],
        [8282689800490735202, 7163455575275556680],
        [9442129501587623646, 11170493004661211510],
        [1804977644159289754, 6447399642706321498],
        [12962882415819734627, 10265229801823221233],
    ],
    [
        [12136670303017559860, 10954863181361510604],
        [16818799500679900733, 26301347801157184],
        [16523734560203373818, 8875706315938308642],
        [13652215662217472305, 14360289746240670232],
        [15997137209224730382, 2855249365085738073],
        [8439036204971282280, 5332314310947209450],
    ],
    [
        [5002312219695870028, 12864539108317097226],
        [11288026870210090771, 537467397206489717],
        [16605383016163841469, 11565142236786528129],
        [648451236574991516, 17133541471825282788],
        [8799253633954543592, 16599237615614283306],
        [14662324510618569403, 15017631772074901142],
    ],
    [
        [1125161078841102465, 16758568561086051281],
        [4583274030668043704, 7639503577039708692],
        [11579888988268885909, 17196824186608462639],
        [17613254186212949568, 7221553970388365607],
        [11809912516148193732, 15218151265935121665],
        [16908672278966459126, 14150948105457976453],
    ],
    [
        [13772217792637998901, 1855258611742756959],
        [2582884916427729101, 2509032786159165530],
        [14125620407098760514, 1504417537277405070],
        [10305323313665474905, 1604382286659379559],
        [14421112102304410128, 14027894372051460183],
        [15443008112223289095, 15897221295594114166],
    ],
    [
        [12072032245098217035, 7191588475441306753],
        [12623124304001543580, 6126399664373147357],
        [15275096536240331360, 13612435704968367041],
        [5776026947445797801, 3889299150961302870],
        [14523998261281656735, 7742614208002775563],
        [16858143668406375798, 3160206292629320261],
    ],
    [
        [10225062286853415962, 4867894860440039506],
        [15230443765010074044, 5834074122870649577],
        [1411277895345654201, 6103669907770093860],
        [9455807740109614010, 2328895372321939992],
        [3950360437029281922, 2759701170044507514],
        [12222443604344363076, 17656869964661373683],
    ],
    [
        [1979172435178892811, 734365595761811365],
        [15025940191449422558, 7227675686012445583],
        [10300025818011524013, 17799497588768886716],
        [2677133735436115112, 14812502602229457013],
        [16627956112613426109, 16752313816123158307],
        [18021340578624713228, 15588028933771063593],
    ],
    [
        [17406042138290610399, 1155452770337571668],
        [13323974833041910639, 2643915518361515739],
        [8461455259953534458, 5962362800473405203],
        [1486328236823662763, 6029292411787016987],
        [4570501748689656135, 13209385274029037864],
        [1640473869064724755, 9993993751204844544],
    ],
    [
        [12421253128466588830, 8322013335805712686],
        [7362065717451500867, 8322398065429370385],
        [17547900980601806984, 16375381847781977787],
        [9978082969672032755, 10576428099498926458],
        [1166295876349253174, 3798598616166945970],
        [10286490312413180567, 286942627933176204],
    ],
    [
        [3002911422422872486, 18440417861633804236],
        [3416197124265049245, 17816331052411515616],
        [10413638395109572301, 5954207306769625314],
        [1469535147040591268, 5164144796376218894],
        [9136547841370018499, 8715315690111987174],
        [9088963607359117003, 5570419926805699874],
    ],
    [
        [13412006813220492616, 15730477022174840098],
        [9212417167932734541, 11933571026463228302],
        [13505266863938557845, 4797511744877234503],
        [7119081543595184306, 11553176455630651762],
        [17435386549912042581, 2143972668496905751],
        [17470561706678163252, 18157862665401097221],
    ],
    [
        [229041617753390049, 14796990186283079237],
        [18429983784475918628, 1291445446240620803],
        [10952493798735693304, 10621929652683581361],
        [7347640877914286689, 1042819558479275492],
        [3197764534496989474, 11861469298543927729],
        [4684694010668199200, 13759548949793093129],
    ],
    [
        [15050270661286871493, 218552656335788818],
        [2270951297827861670, 15531027559614454117],
        [7771650336223550529, 3307223617244388662],
        [11694453156932746226, 18076894744685470676],
        [3102641096343005462, 7063833182539741922],
        [16294365620707532172, 4028165365651975745],
    ],
];
